<script setup lang="ts">
import { computed, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { CloseOutlined, FullscreenExitOutlined, FullscreenOutlined } from '@ant-design/icons-vue'
import { useReaderStore } from '../../stores/reader'
import { useReaderExercise } from '../../composables/useReaderExercise'
import {
  useExerciseDiagnostics,
  type ExerciseDebugMeta,
} from '../../composables/reader/useExerciseDiagnostics'
import { useExerciseModalGeometry } from '../../composables/reader/useExerciseModalGeometry'
import {
  extractExerciseRuntimePaths,
  type ExerciseRuntimeContext,
} from '../../lib/exercise/runtime'

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
const runtimePaths = computed(() => extractExerciseRuntimePaths(currentExerciseHtml.value))
const showDebugPanel = computed(() => props.enableDebugPanel)
const iframeSandbox = computed(
  () => 'allow-scripts allow-same-origin allow-forms allow-modals allow-popups',
)
const activeExerciseSrc = computed(() => currentExerciseUrl.value || '')
const debugMeta = computed(() => props.debugMeta)
const contextRef = computed<ExerciseRuntimeContext>(() => ({
  resourceId: currentExerciseResourceId.value || currentExerciseTitle.value || 'exercise',
  title: currentExerciseTitle.value,
  paths: runtimePaths.value,
}))

const {
  isPhone,
  isMaximized,
  modalWidth,
  modalBodyHeight,
  modalStyle,
  canResize,
  startDragging,
  startResizing,
  closeModal,
  toggleMaximize,
} = useExerciseModalGeometry(exerciseVisible)

const {
  logPanelRef,
  debugLogLines,
  debugLogText,
  isCopyingLogs,
  appendDebugLog,
  clearDebugLogs,
  copyDebugLogs,
  handleIframeLoad,
  handleIframeError,
} = useExerciseDiagnostics({
  showDebugPanel,
  exerciseVisible,
  iframeRef,
  currentExerciseUrl,
  currentExerciseHtml,
  currentExerciseTitle,
  currentExerciseResourceId,
  activeExerciseSrc,
  runtimePaths,
  iframeSandbox,
  debugMeta,
})

useReaderExercise({
  iframeRef,
  contextRef,
  enabledRef: exerciseVisible,
  onBridgeLog: (level, message, payload) => {
    appendDebugLog(level, `[ExerciseBridge] ${message}`, payload)
  },
})

const debugPanelHeight = computed(() => (showDebugPanel.value ? (isPhone.value ? 150 : 190) : 0))
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
        class="flex h-10 cursor-move touch-none select-none items-center justify-between bg-slate-50 px-2"
        @pointerdown="startDragging"
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
            @pointerdown.stop
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
            @pointerdown.stop
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
