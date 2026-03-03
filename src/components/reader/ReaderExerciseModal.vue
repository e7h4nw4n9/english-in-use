<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { CloseOutlined, FullscreenExitOutlined, FullscreenOutlined } from '@ant-design/icons-vue'
import { useReaderStore } from '../../stores/reader'
import { useReaderExercise } from '../../composables/useReaderExercise'
import {
  extractExerciseRuntimePaths,
  type ExerciseRuntimeContext,
} from '../../lib/exercise/runtime'

interface ExerciseDebugMeta {
  productCode?: string
  pageLabel?: string
  unitName?: string
}

const props = withDefaults(
  defineProps<{
    enableDebugPanel?: boolean
    debugMeta?: ExerciseDebugMeta
  }>(),
  {
    enableDebugPanel: false,
    debugMeta: () => ({}),
  },
)

const readerStore = useReaderStore()
const {
  exerciseVisible,
  currentExerciseUrl,
  currentExerciseHtml,
  currentExerciseTitle,
  currentExerciseResourceId,
} = storeToRefs(readerStore)

const iframeRef = ref<HTMLIFrameElement | null>(null)
const logPanelRef = ref<HTMLElement | null>(null)
const debugLogLines = ref<string[]>([])
const isCopyingLogs = ref(false)

const MAX_DEBUG_LOG_LINES = 220
const MAX_DEBUG_PAYLOAD_CHARS = 1400
const MAX_BACKEND_LOG_FILES = 2
const MAX_BACKEND_LOG_CHARS_PER_FILE = 120_000
const BACKEND_LOG_FALLBACK_NAMES = [
  'app.log',
  'app.log.1',
  'app.log.2',
  'app',
  'app.txt',
  'app-0.log',
  'app-1.log',
]

const runtimePaths = computed(() => extractExerciseRuntimePaths(currentExerciseHtml.value))
const showDebugPanel = computed(() => props.enableDebugPanel)
const iframeSandbox = computed(() =>
  showDebugPanel.value
    ? undefined
    : 'allow-scripts allow-same-origin allow-forms allow-modals allow-popups',
)
const debugPanelHeight = computed(() => (showDebugPanel.value ? (isPhone.value ? 150 : 190) : 0))
const debugLogText = computed(() => debugLogLines.value.join('\n'))
const activeExerciseSrc = computed(() => currentExerciseUrl.value || '')

const contextRef = computed<ExerciseRuntimeContext>(() => ({
  resourceId: currentExerciseResourceId.value || currentExerciseTitle.value || 'exercise',
  title: currentExerciseTitle.value,
  paths: runtimePaths.value,
}))

useReaderExercise({
  iframeRef,
  contextRef,
  enabledRef: exerciseVisible,
  onBridgeLog: (level, message, payload) => {
    appendDebugLog(level, `[ExerciseBridge] ${message}`, payload)
  },
})

function payloadToText(payload: unknown): string {
  if (payload === undefined) return ''
  try {
    const serialized = JSON.stringify(payload)
    if (serialized.length <= MAX_DEBUG_PAYLOAD_CHARS) {
      return serialized
    }
    return `${serialized.slice(0, MAX_DEBUG_PAYLOAD_CHARS)}...<truncated>`
  } catch {
    const fallback = String(payload)
    if (fallback.length <= MAX_DEBUG_PAYLOAD_CHARS) {
      return fallback
    }
    return `${fallback.slice(0, MAX_DEBUG_PAYLOAD_CHARS)}...<truncated>`
  }
}

function scrollLogPanelToBottom() {
  if (!showDebugPanel.value) return
  if (typeof window === 'undefined') return
  window.requestAnimationFrame(() => {
    if (!logPanelRef.value) return
    logPanelRef.value.scrollTop = logPanelRef.value.scrollHeight
  })
}

function appendDebugLog(level: 'info' | 'error', message: string, payload?: unknown) {
  if (!showDebugPanel.value) return ''
  const payloadText = payload === undefined ? '' : ` | payload=${payloadToText(payload)}`
  const timestamp = new Date().toISOString()
  const line = `[${timestamp}] [${level.toUpperCase()}] ${message}${payloadText}`
  if (debugLogLines.value.length >= MAX_DEBUG_LOG_LINES) {
    debugLogLines.value = [...debugLogLines.value.slice(-(MAX_DEBUG_LOG_LINES - 1)), line]
  } else {
    debugLogLines.value = [...debugLogLines.value, line]
  }
  scrollLogPanelToBottom()
  return line
}

function clearDebugLogs() {
  debugLogLines.value = []
}

async function readKnownBackendLogs(
  readTextFile: (path: string, options: { baseDir: number }) => Promise<string>,
  appLogBaseDir: number,
) {
  const sections: string[] = []
  for (const name of BACKEND_LOG_FALLBACK_NAMES) {
    try {
      const fullText = await readTextFile(name, { baseDir: appLogBaseDir })
      const text =
        fullText.length <= MAX_BACKEND_LOG_CHARS_PER_FILE
          ? fullText
          : `...<truncated head>\n${fullText.slice(-MAX_BACKEND_LOG_CHARS_PER_FILE)}`
      sections.push(
        [
          `backend_file: ${name}`,
          `backend_file_chars: ${fullText.length}`,
          'backend_file_content_begin',
          text,
          'backend_file_content_end',
        ].join('\n'),
      )
    } catch {
      // Ignore missing files in fallback probing.
    }
  }
  return sections
}

async function buildBackendLogSection() {
  try {
    const [{ readDir, readTextFile, stat }, { BaseDirectory }] = await Promise.all([
      import('@tauri-apps/plugin-fs'),
      import('@tauri-apps/api/path'),
    ])
    let entries: Awaited<ReturnType<typeof readDir>> = []
    try {
      entries = await readDir('', { baseDir: BaseDirectory.AppLog })
    } catch (error) {
      const fallbackSections = await readKnownBackendLogs(readTextFile, BaseDirectory.AppLog)
      if (fallbackSections.length > 0) {
        return [
          'backend_logs_note: readDir failed, used known file name fallback',
          `backend_logs_note_error: ${String(error)}`,
          '',
          fallbackSections.join('\n\n'),
        ].join('\n')
      }
      return `backend_logs_unavailable: ${String(error)}\n`
    }

    const candidates = entries.filter((entry) => entry.isFile && /^app(\.|-|$)/i.test(entry.name))
    if (!candidates.length) {
      const fallbackSections = await readKnownBackendLogs(readTextFile, BaseDirectory.AppLog)
      if (fallbackSections.length > 0) {
        return fallbackSections.join('\n\n')
      }
      return 'backend_logs: none (no app*.log files found)\n'
    }

    const withStats = await Promise.all(
      candidates.map(async (entry) => {
        try {
          const info = await stat(entry.name, { baseDir: BaseDirectory.AppLog })
          const mtime = info.mtime ? new Date(info.mtime).getTime() : 0
          return { name: entry.name, mtime }
        } catch {
          return { name: entry.name, mtime: 0 }
        }
      }),
    )

    const sorted = withStats
      .sort((a, b) => b.mtime - a.mtime || a.name.localeCompare(b.name))
      .slice(0, MAX_BACKEND_LOG_FILES)

    const sections: string[] = []
    for (const item of sorted) {
      try {
        const fullText = await readTextFile(item.name, { baseDir: BaseDirectory.AppLog })
        const text =
          fullText.length <= MAX_BACKEND_LOG_CHARS_PER_FILE
            ? fullText
            : `...<truncated head>\n${fullText.slice(-MAX_BACKEND_LOG_CHARS_PER_FILE)}`
        sections.push(
          [
            `backend_file: ${item.name}`,
            `backend_file_chars: ${fullText.length}`,
            'backend_file_content_begin',
            text,
            'backend_file_content_end',
          ].join('\n'),
        )
      } catch (error) {
        sections.push(
          [`backend_file: ${item.name}`, `backend_file_error: ${String(error)}`].join('\n'),
        )
      }
    }

    return sections.join('\n\n')
  } catch (error) {
    return `backend_logs_unavailable: ${String(error)}\n`
  }
}

async function copyDebugLogs() {
  if (!showDebugPanel.value) return
  if (isCopyingLogs.value) return
  isCopyingLogs.value = true
  void logModalEvent('info', 'debug log copy started')

  try {
    const backendLogSection = await buildBackendLogSection()
    const copyText = [
      '# exercise iframe debug copy',
      `generated_at: ${new Date().toISOString()}`,
      `product_code: ${props.debugMeta.productCode || ''}`,
      `page_label: ${props.debugMeta.pageLabel || ''}`,
      `unit_name: ${props.debugMeta.unitName || ''}`,
      `resource_id: ${currentExerciseResourceId.value || ''}`,
      `exercise_title: ${currentExerciseTitle.value || ''}`,
      `active_src: ${activeExerciseSrc.value}`,
      `user_agent: ${typeof navigator !== 'undefined' ? navigator.userAgent : ''}`,
      '',
      '## frontend_log_lines',
      debugLogText.value || '[no logs]',
      '',
      '## backend_log_files',
      backendLogSection,
      '',
    ].join('\n')

    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(copyText)
      void logModalEvent('info', 'debug log copied to clipboard', {
        chars: copyText.length,
      })
      return
    }

    void logModalEvent('error', 'debug log copy failed', {
      reason: 'clipboard API unavailable',
    })
  } catch (error) {
    void logModalEvent('error', 'debug log copy failed', {
      error: String(error),
    })
  } finally {
    isCopyingLogs.value = false
  }
}

const logModalEvent = async (level: 'info' | 'error', message: string, payload?: unknown) => {
  if (!showDebugPanel.value) return
  const payloadText = payload === undefined ? '' : ` | payload=${payloadToText(payload)}`
  const line = `[ExerciseModal] ${message}${payloadText}`
  appendDebugLog(level, line)

  try {
    const logger = await import('@tauri-apps/plugin-log')
    if (level === 'error') {
      await logger.error(line)
    } else {
      await logger.info(line)
    }
  } catch {
    if (level === 'error') {
      console.error(line)
    } else {
      console.info(line)
    }
  }
}

function handleIframeLoad() {
  void logModalEvent('info', 'iframe load event triggered')
}

function handleIframeError() {
  void logModalEvent('error', 'iframe error', {
    hasUrlSrc: Boolean(currentExerciseUrl.value),
  })
}

function isFromExerciseIframe(source: MessageEventSource | null): boolean {
  return Boolean(iframeRef.value?.contentWindow && source === iframeRef.value.contentWindow)
}

function handleWindowMessage(event: MessageEvent) {
  if (!showDebugPanel.value) return
  if (!exerciseVisible.value) return

  // Try to parse message regardless of strict source check if it looks like our log type
  let parsedData: any = null
  try {
    parsedData = typeof event.data === 'string' ? JSON.parse(event.data) : event.data
  } catch (e) {}

  if (!isFromExerciseIframe(event.source)) {
    // If it's a runtime log but source check failed (can happen in some WebView versions),
    // allow it if the shape matches
    if (parsedData?.type === 'eiu-exercise-runtime-log') {
      const level = typeof parsedData.level === 'string' ? parsedData.level : 'info'
      void logModalEvent(level as any, `bridge: iframe ${level}`, parsedData.payload)
    }
    return
  }

  const data = parsedData || event.data
  const payload =
    typeof data === 'string'
      ? {
          type: 'string',
          preview: data.slice(0, 180),
        }
      : data && typeof data === 'object'
        ? {
            type: String((data as Record<string, unknown>).type || 'object'),
            keys: Object.keys(data as Record<string, unknown>).slice(0, 10),
          }
        : {
            type: typeof data,
            preview: String(data),
          }

  void logModalEvent('info', 'iframe window message', {
    origin: event.origin || '',
    ...payload,
  })
}

function handleWindowError(event: ErrorEvent) {
  if (!showDebugPanel.value) return
  if (!exerciseVisible.value) return
  void logModalEvent('error', 'window error while exercise visible', {
    message: event.message,
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno,
  })
}

function handleWindowUnhandledRejection(event: PromiseRejectionEvent) {
  if (!showDebugPanel.value) return
  if (!exerciseVisible.value) return
  void logModalEvent('error', 'window unhandled rejection while exercise visible', {
    reason: String(event.reason),
  })
}

const TOOLBAR_HEIGHT = 40

const viewportWidth = ref(typeof window !== 'undefined' ? window.innerWidth : 1280)
const viewportHeight = ref(typeof window !== 'undefined' ? window.innerHeight : 800)
const isMaximized = ref(false)
const modalPosition = ref({ x: 0, y: 0 })
const lastNormalPosition = ref<{ x: number; y: number } | null>(null)
const manualSize = ref<{ width: number; height: number } | null>(null)

const dragState = ref({
  dragging: false,
  startX: 0,
  startY: 0,
  originX: 0,
  originY: 0,
})
const resizeState = ref({
  resizing: false,
  startX: 0,
  startY: 0,
  originWidth: 0,
  originHeight: 0,
  pointerId: null as number | null,
})

const isPhone = computed(() => viewportWidth.value < 768)
const isTablet = computed(() => viewportWidth.value >= 768 && viewportWidth.value < 1100)
const modalMarginRatio = computed(() => (isPhone.value ? 0.035 : isTablet.value ? 0.04 : 0.05))
const modalMargin = computed(() =>
  Math.round(Math.min(viewportWidth.value, viewportHeight.value) * modalMarginRatio.value),
)
const availableWidth = computed(() => Math.max(0, viewportWidth.value - modalMargin.value * 2))
const availableHeight = computed(() => Math.max(0, viewportHeight.value - modalMargin.value * 2))
const minModalWidth = computed(() => availableWidth.value * (isPhone.value ? 0.88 : 0.25))
const minModalHeight = computed(() => availableHeight.value * (isPhone.value ? 0.56 : 0.42))
const defaultModalWidth = computed(() => availableWidth.value * (isPhone.value ? 1 : 1 / 3))
const defaultModalHeight = computed(() => {
  const heightRatio = isPhone.value ? 0.8 : isTablet.value ? 0.72 : 0.66
  return availableHeight.value * heightRatio
})
const canResize = computed(() => !isPhone.value && !isMaximized.value)

const modalWidth = computed(() => {
  if (isMaximized.value) {
    return availableWidth.value
  }
  const preferredWidth = manualSize.value?.width ?? defaultModalWidth.value
  return Math.min(Math.max(preferredWidth, minModalWidth.value), availableWidth.value)
})

const modalHeight = computed(() => {
  if (isMaximized.value) {
    return availableHeight.value
  }
  const preferredHeight = manualSize.value?.height ?? defaultModalHeight.value
  return Math.min(Math.max(preferredHeight, minModalHeight.value), availableHeight.value)
})

const modalBodyHeight = computed(() => `${Math.max(0, modalHeight.value - TOOLBAR_HEIGHT)}px`)

const modalStyle = computed(() => ({
  top: `${modalPosition.value.y}px`,
  left: `${modalPosition.value.x}px`,
  margin: '0',
  paddingBottom: '0',
  position: 'fixed' as const,
}))

function clampPosition(x: number, y: number) {
  const margin = modalMargin.value
  const maxX = Math.max(margin, viewportWidth.value - modalWidth.value - margin)
  const maxY = Math.max(margin, viewportHeight.value - modalHeight.value - margin)
  const clampedX = Math.min(Math.max(margin, x), maxX)
  const clampedY = Math.min(Math.max(margin, y), maxY)
  return { x: clampedX, y: clampedY }
}

function centerModal() {
  const centeredX = (viewportWidth.value - modalWidth.value) / 2
  const centeredY = (viewportHeight.value - modalHeight.value) / 2
  modalPosition.value = clampPosition(centeredX, centeredY)
}

function stopDragging() {
  if (!dragState.value.dragging) return
  dragState.value.dragging = false
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', stopDragging)
}

function stopResizing(event?: PointerEvent) {
  if (!resizeState.value.resizing) return
  if (
    event &&
    resizeState.value.pointerId !== null &&
    event.pointerId !== resizeState.value.pointerId
  ) {
    return
  }
  resizeState.value.resizing = false
  resizeState.value.pointerId = null
  window.removeEventListener('pointermove', onResizePointerMove)
  window.removeEventListener('pointerup', stopResizing)
  window.removeEventListener('pointercancel', stopResizing)
}

function onResizePointerMove(event: PointerEvent) {
  if (!resizeState.value.resizing) return
  if (resizeState.value.pointerId !== null && event.pointerId !== resizeState.value.pointerId) {
    return
  }

  event.preventDefault()
  const deltaX = event.clientX - resizeState.value.startX
  const deltaY = event.clientY - resizeState.value.startY
  const nextWidth = resizeState.value.originWidth + deltaX
  const nextHeight = resizeState.value.originHeight + deltaY
  const width = Math.min(Math.max(nextWidth, minModalWidth.value), availableWidth.value)
  const height = Math.min(Math.max(nextHeight, minModalHeight.value), availableHeight.value)
  manualSize.value = { width, height }
  modalPosition.value = clampPosition(modalPosition.value.x, modalPosition.value.y)
}

function startResizing(event: PointerEvent) {
  if (!canResize.value || dragState.value.dragging) return
  if (event.pointerType === 'mouse' && event.button !== 0) return

  event.preventDefault()
  resizeState.value = {
    resizing: true,
    startX: event.clientX,
    startY: event.clientY,
    originWidth: modalWidth.value,
    originHeight: modalHeight.value,
    pointerId: event.pointerId,
  }

  window.addEventListener('pointermove', onResizePointerMove, { passive: false })
  window.addEventListener('pointerup', stopResizing)
  window.addEventListener('pointercancel', stopResizing)
}

function onMouseMove(event: MouseEvent) {
  if (!dragState.value.dragging || isMaximized.value) return
  const deltaX = event.clientX - dragState.value.startX
  const deltaY = event.clientY - dragState.value.startY
  modalPosition.value = clampPosition(
    dragState.value.originX + deltaX,
    dragState.value.originY + deltaY,
  )
}

function startDragging(event: MouseEvent) {
  if (isMaximized.value || resizeState.value.resizing || event.button !== 0) return
  dragState.value = {
    dragging: true,
    startX: event.clientX,
    startY: event.clientY,
    originX: modalPosition.value.x,
    originY: modalPosition.value.y,
  }
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', stopDragging)
}

function closeModal() {
  exerciseVisible.value = false
}

function toggleMaximize() {
  if (!isMaximized.value) {
    lastNormalPosition.value = { ...modalPosition.value }
    stopResizing()
    isMaximized.value = true
    modalPosition.value = { x: modalMargin.value, y: modalMargin.value }
    return
  }

  isMaximized.value = false
  if (lastNormalPosition.value) {
    modalPosition.value = clampPosition(lastNormalPosition.value.x, lastNormalPosition.value.y)
  } else {
    centerModal()
  }
}

function onWindowResize() {
  viewportWidth.value = window.innerWidth
  viewportHeight.value = window.innerHeight

  if (manualSize.value) {
    manualSize.value = {
      width: Math.min(Math.max(manualSize.value.width, minModalWidth.value), availableWidth.value),
      height: Math.min(
        Math.max(manualSize.value.height, minModalHeight.value),
        availableHeight.value,
      ),
    }
  }

  if (isMaximized.value) {
    modalPosition.value = { x: modalMargin.value, y: modalMargin.value }
    return
  }
  modalPosition.value = clampPosition(modalPosition.value.x, modalPosition.value.y)
}

function onWindowKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && exerciseVisible.value) {
    closeModal()
  }
}

watch(
  () => exerciseVisible.value,
  (visible) => {
    if (visible) {
      clearDebugLogs()
      if (showDebugPanel.value) {
        void logModalEvent('info', 'visible=true', {
          hasHtml: Boolean(currentExerciseHtml.value),
          htmlLength: currentExerciseHtml.value.length,
          activeSrcPrefix: activeExerciseSrc.value.slice(0, 140),
          hasEnginePath: Boolean(runtimePaths.value.engine),
          hasDpPath: Boolean(runtimePaths.value.dp),
          enginePathPrefix: runtimePaths.value.engine
            ? runtimePaths.value.engine.slice(0, 140)
            : '',
          dpPathPrefix: runtimePaths.value.dp ? runtimePaths.value.dp.slice(0, 140) : '',
          productCode: props.debugMeta.productCode || '',
          pageLabel: props.debugMeta.pageLabel || '',
          unitName: props.debugMeta.unitName || '',
          sandboxDisabledForDebug: !iframeSandbox.value,
          userAgent: typeof navigator !== 'undefined' ? navigator.userAgent : '',
        })
      }
      isMaximized.value = false
      stopDragging()
      stopResizing()
      centerModal()
      return
    }
    if (showDebugPanel.value) {
      void logModalEvent('info', 'visible=false')
    }
    stopDragging()
    stopResizing()
  },
)

watch(
  activeExerciseSrc,
  (src) => {
    if (!showDebugPanel.value) return
    if (!src) return
    void logModalEvent('info', 'active iframe src changed', {
      sourceType: 'url',
      srcPrefix: src.slice(0, 180),
    })
  },
  { immediate: true },
)

onMounted(() => {
  window.addEventListener('resize', onWindowResize)
  window.addEventListener('keydown', onWindowKeydown)
  window.addEventListener('message', handleWindowMessage)
  window.addEventListener('error', handleWindowError)
  window.addEventListener('unhandledrejection', handleWindowUnhandledRejection)
})

onBeforeUnmount(() => {
  stopDragging()
  stopResizing()
  window.removeEventListener('resize', onWindowResize)
  window.removeEventListener('keydown', onWindowKeydown)
  window.removeEventListener('message', handleWindowMessage)
  window.removeEventListener('error', handleWindowError)
  window.removeEventListener('unhandledrejection', handleWindowUnhandledRejection)
})
</script>

<template>
  <a-modal
    v-model:open="exerciseVisible"
    wrap-class-name="exercise-modal-wrap"
    :title="null"
    :closable="false"
    :style="modalStyle"
    :body-style="{ padding: '0' }"
    :width="modalWidth"
    :footer="null"
    :mask-closable="false"
    destroy-on-close
    @cancel="closeModal"
  >
    <div class="relative w-full overflow-hidden rounded-lg bg-white">
      <div
        class="flex h-10 cursor-move select-none items-center justify-between bg-slate-50 px-2"
        @mousedown="startDragging"
      >
        <div class="flex min-w-0 items-center gap-2 overflow-hidden text-[11px] text-slate-600">
          <span class="max-w-[180px] truncate font-medium text-slate-700">
            {{ currentExerciseTitle || 'Exercise' }}
          </span>
          <span class="truncate rounded bg-slate-200/80 px-1 py-0.5 font-mono">
            {{ currentExerciseResourceId || '-' }}
          </span>
          <span v-if="showDebugPanel" class="rounded bg-amber-100 px-1 py-0.5 text-amber-700">
            iframe logs
          </span>
        </div>
        <div class="flex items-center gap-2">
          <button
            data-testid="exercise-maximize"
            type="button"
            class="inline-flex h-6 w-6 items-center justify-center rounded bg-white text-slate-600 hover:bg-slate-100"
            :aria-label="isMaximized ? 'Restore' : 'Maximize'"
            @mousedown.stop
            @click="toggleMaximize"
          >
            <FullscreenExitOutlined v-if="isMaximized" />
            <FullscreenOutlined v-else />
          </button>
          <button
            data-testid="exercise-close"
            type="button"
            class="inline-flex h-6 w-6 items-center justify-center rounded bg-white text-slate-600 hover:bg-slate-100"
            aria-label="Close"
            @mousedown.stop
            @click="closeModal"
          >
            <CloseOutlined />
          </button>
        </div>
      </div>

      <div :style="{ height: modalBodyHeight }" class="flex w-full flex-col">
        <div class="min-h-0 flex-1">
          <iframe
            v-if="currentExerciseUrl"
            ref="iframeRef"
            :src="currentExerciseUrl"
            class="h-full w-full border-none"
            allow="autoplay; fullscreen; clipboard-read; clipboard-write"
            :sandbox="iframeSandbox"
            @load="handleIframeLoad"
            @error="handleIframeError"
          ></iframe>
          <div
            v-else
            class="flex h-full items-center justify-center bg-slate-100 text-xs text-slate-500"
          >
            No exercise source.
          </div>
        </div>

        <div
          v-if="showDebugPanel"
          :style="{ height: `${debugPanelHeight}px` }"
          class="flex flex-col border-t border-slate-700 bg-slate-900 text-slate-200"
        >
          <div class="flex h-7 items-center justify-between px-2 text-[10px]">
            <span class="font-semibold tracking-wide text-slate-100">Exercise iframe logs</span>
            <div class="flex items-center gap-3">
              <span class="font-mono text-slate-400">{{ debugLogLines.length }} lines</span>
              <button
                type="button"
                class="rounded border border-slate-600 px-1.5 py-0.5 text-slate-300 hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-60"
                :disabled="isCopyingLogs"
                @click="copyDebugLogs"
              >
                {{ isCopyingLogs ? 'Copying...' : 'Copy' }}
              </button>
              <button
                type="button"
                class="rounded border border-slate-600 px-1.5 py-0.5 text-slate-300 hover:bg-slate-800"
                @click="clearDebugLogs"
              >
                Clear
              </button>
            </div>
          </div>
          <pre
            ref="logPanelRef"
            class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap px-2 pb-2 font-mono text-[10px] leading-4 text-slate-200"
            >{{ debugLogText || '[no logs yet]' }}</pre
          >
        </div>
      </div>

      <button
        v-if="canResize"
        data-testid="exercise-resize"
        type="button"
        class="absolute bottom-0 right-0 inline-flex h-5 w-5 cursor-se-resize touch-none items-end justify-end bg-transparent text-slate-400"
        aria-label="Resize"
        @pointerdown="startResizing"
      >
        <span class="mb-[2px] mr-[2px] block h-2.5 w-2.5 border-b border-r border-slate-300"></span>
      </button>
    </div>
  </a-modal>
</template>

<style>
.exercise-modal-wrap .ant-modal {
  padding-bottom: 0 !important;
}

.exercise-modal-wrap .ant-modal-content {
  border: none;
  box-shadow: none;
  overflow: hidden;
  padding: 0 !important;
}

.exercise-modal-wrap .ant-modal-body {
  padding: 0 !important;
}
</style>
