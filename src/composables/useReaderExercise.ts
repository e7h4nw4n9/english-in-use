import { onUnmounted, watch, type Ref } from 'vue'
import { createExerciseBridge, type ExerciseRuntimeContext } from '@/lib/exercise/runtime'

interface UseReaderExerciseOptions {
  iframeRef: Ref<HTMLIFrameElement | null>
  contextRef: Ref<ExerciseRuntimeContext>
  enabledRef: Ref<boolean>
  onBridgeLog?: (level: 'info' | 'error', message: string, payload?: unknown) => void
}

function bridgeLogLevel(message: string): 'info' | 'error' {
  return /\berror\b|unhandledrejection|failed/i.test(message) ? 'error' : 'info'
}

/**
 * 管理练习 iframe 与运行时桥接的绑定和清理。
 * @param options - 练习 iframe、运行上下文、启用状态和日志回调。
 */
export function useReaderExercise({
  iframeRef,
  contextRef,
  enabledRef,
  onBridgeLog,
}: UseReaderExerciseOptions) {
  let detachBridge: (() => void) | null = null

  const logBridgeEvent = async (message: string, payload?: unknown) => {
    const payloadText =
      payload === undefined
        ? ''
        : ` | payload=${(() => {
            try {
              return JSON.stringify(payload)
            } catch {
              return String(payload)
            }
          })()}`
    const line = `[ExerciseBridge] ${message}${payloadText}`
    const level = bridgeLogLevel(message)
    onBridgeLog?.(level, message, payload)
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

  const bindBridge = () => {
    if (detachBridge) {
      detachBridge()
      detachBridge = null
    }

    if (!enabledRef.value) {
      void logBridgeEvent('bridge: bind skipped', {
        enabled: enabledRef.value,
        hasIframe: Boolean(iframeRef.value),
      })
      return
    }

    if (!iframeRef.value) {
      void logBridgeEvent('bridge: bind deferred', {
        resourceId: contextRef.value.resourceId,
        title: contextRef.value.title || '',
        hasEnginePath: Boolean(contextRef.value.paths?.engine),
        hasDpPath: Boolean(contextRef.value.paths?.dp),
      })
      return
    }

    void logBridgeEvent('bridge: bind start', {
      resourceId: contextRef.value.resourceId,
      title: contextRef.value.title || '',
      hasEnginePath: Boolean(contextRef.value.paths?.engine),
      hasDpPath: Boolean(contextRef.value.paths?.dp),
    })

    detachBridge = createExerciseBridge({
      iframe: iframeRef.value,
      getContext: () => contextRef.value,
      onLog: (message, payload) => {
        void logBridgeEvent(message, payload)
      },
    })
  }

  watch([iframeRef, contextRef, enabledRef], bindBridge, {
    immediate: true,
    flush: 'post',
  })

  onUnmounted(() => {
    if (detachBridge) {
      detachBridge()
      detachBridge = null
    }
  })

  return {}
}
