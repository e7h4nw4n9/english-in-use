import { computed, onBeforeUnmount, onMounted, ref, watch, type ComputedRef, type Ref } from 'vue'
import type { ExerciseRuntimePaths } from '../../lib/exercise/runtime'

export interface ExerciseDebugMeta {
  productCode?: string
  pageLabel?: string
  unitName?: string
}

interface ExerciseDiagnosticsOptions {
  showDebugPanel: ComputedRef<boolean>
  exerciseVisible: Ref<boolean>
  iframeRef: Ref<HTMLIFrameElement | null>
  currentExerciseUrl: Ref<string>
  currentExerciseHtml: Ref<string>
  currentExerciseTitle: Ref<string>
  currentExerciseResourceId: Ref<string>
  activeExerciseSrc: ComputedRef<string>
  runtimePaths: ComputedRef<ExerciseRuntimePaths>
  iframeSandbox: ComputedRef<string>
  debugMeta: ComputedRef<ExerciseDebugMeta>
}

/**
 * 管理练习 iframe 的诊断日志、后端日志复制和窗口错误监听。
 * @param options - 练习状态、iframe 引用和调试上下文。
 */
export function useExerciseDiagnostics(options: ExerciseDiagnosticsOptions) {
  const {
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
  } = options
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
  const debugLogText = computed(() => debugLogLines.value.join('\n'))

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

  /** 从已知日志目录读取指定后端日志文件。
   * @param fileName - 日志文件名。
   * @param maxChars - 最多保留的尾部字符数。
   */
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
        // 回退探测时允许候选日志文件不存在。
      }
    }
    return sections
  }

  /** 汇总练习、前端和后端日志，供复制诊断信息使用。 */
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
        `product_code: ${debugMeta.value.productCode || ''}`,
        `page_label: ${debugMeta.value.pageLabel || ''}`,
        `unit_name: ${debugMeta.value.unitName || ''}`,
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
    if (import.meta.env.MODE === 'test') return

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

  /** 判断消息来源是否为当前练习 iframe。
   * @param source - 浏览器消息来源窗口。
   */
  function isFromExerciseIframe(source: MessageEventSource | null): boolean {
    return Boolean(iframeRef.value?.contentWindow && source === iframeRef.value.contentWindow)
  }

  /** 接收练习 iframe 运行日志，并拒绝其他窗口发送的消息。
   * @param event - 浏览器消息事件。
   */
  function handleWindowMessage(event: MessageEvent) {
    if (!showDebugPanel.value) return
    if (!exerciseVisible.value) return

    if (!isFromExerciseIframe(event.source)) {
      return
    }

    let parsedData: unknown = null
    try {
      parsedData = typeof event.data === 'string' ? JSON.parse(event.data) : event.data
    } catch {
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

  /** 捕获练习弹窗作用域内的未处理脚本错误。
   * @param event - 浏览器错误事件。
   */
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

  /** 捕获练习弹窗作用域内的未处理 Promise 拒绝。
   * @param event - Promise 拒绝事件。
   */
  function handleWindowUnhandledRejection(event: PromiseRejectionEvent) {
    if (!showDebugPanel.value) return
    if (!exerciseVisible.value) return
    void logModalEvent('error', 'window unhandled rejection while exercise visible', {
      reason: String(event.reason),
    })
  }

  watch(exerciseVisible, (visible) => {
    if (visible) {
      clearDebugLogs()
      if (showDebugPanel.value) {
        void logModalEvent('info', 'visible=true', {
          hasHtml: Boolean(currentExerciseHtml.value),
          htmlLength: currentExerciseHtml.value.length,
          activeSrcPrefix: activeExerciseSrc.value.slice(0, 140),
          hasEnginePath: Boolean(runtimePaths.value.engine),
          hasDpPath: Boolean(runtimePaths.value.dp),
          enginePathPrefix: runtimePaths.value.engine?.slice(0, 140) || '',
          dpPathPrefix: runtimePaths.value.dp?.slice(0, 140) || '',
          productCode: debugMeta.value.productCode || '',
          pageLabel: debugMeta.value.pageLabel || '',
          unitName: debugMeta.value.unitName || '',
          sandboxDisabledForDebug: !iframeSandbox.value,
          userAgent: typeof navigator !== 'undefined' ? navigator.userAgent : '',
        })
      }
    } else if (showDebugPanel.value) {
      void logModalEvent('info', 'visible=false')
    }
  })

  watch(
    activeExerciseSrc,
    (src) => {
      if (!showDebugPanel.value || !src) return
      void logModalEvent('info', 'active iframe src changed', {
        sourceType: 'url',
        srcPrefix: src.slice(0, 180),
      })
    },
    { immediate: true },
  )

  onMounted(() => {
    window.addEventListener('message', handleWindowMessage)
    window.addEventListener('error', handleWindowError)
    window.addEventListener('unhandledrejection', handleWindowUnhandledRejection)
  })

  onBeforeUnmount(() => {
    window.removeEventListener('message', handleWindowMessage)
    window.removeEventListener('error', handleWindowError)
    window.removeEventListener('unhandledrejection', handleWindowUnhandledRejection)
  })

  return {
    logPanelRef,
    debugLogLines,
    debugLogText,
    isCopyingLogs,
    appendDebugLog,
    clearDebugLogs,
    copyDebugLogs,
    handleIframeLoad,
    handleIframeError,
  }
}
