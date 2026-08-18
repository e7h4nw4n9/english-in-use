import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { message as antMessage } from 'ant-design-vue'
import type { Ref } from 'vue'
import { getExerciseHtml } from '@/lib/api/books'
import { getReadableCommandError } from '@/lib/error'
import { extractExerciseRuntimePaths } from '@/lib/exercise/runtime'
import type { Book, ExerciseDownloadProgressEvent, ExerciseInfo } from '@/types'
import type { GlobalLoadingToken } from '@/stores/app'

interface ExerciseAppStore {
  startGlobalLoading: (message?: string) => GlobalLoadingToken
  setGlobalLoadingProgress: (token: GlobalLoadingToken, progress: number | null) => void
  setGlobalLoadingMessage: (token: GlobalLoadingToken, message: string) => void
  stopGlobalLoading: (token: GlobalLoadingToken) => void
}

interface UseReaderExerciseLoaderOptions {
  currentBook: Ref<Book | null>
  appStore: ExerciseAppStore
  exerciseVisible: Ref<boolean>
  currentExerciseUrl: Ref<string>
  currentExerciseHtml: Ref<string>
  currentExerciseTitle: Ref<string>
  currentExerciseResourceId: Ref<string>
  t: (key: string) => string
}

/**
 * 协调练习依赖下载、HTML 加载、进度展示和错误反馈。
 * @param options - 当前图书、练习状态、全局加载状态和翻译函数。
 */
export function useReaderExerciseLoader({
  currentBook,
  appStore,
  exerciseVisible,
  currentExerciseUrl,
  currentExerciseHtml,
  currentExerciseTitle,
  currentExerciseResourceId,
  t,
}: UseReaderExerciseLoaderOptions) {
  let openingExercise = false

  const logExerciseEvent = async (level: 'info' | 'error', message: string, payload?: unknown) => {
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
    const line = `[ExerciseLoader] ${message}${payloadText}`
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

  async function openExercise(exercise: ExerciseInfo) {
    if (!currentBook.value || openingExercise) {
      void logExerciseEvent('info', 'open skipped', {
        hasBook: Boolean(currentBook.value),
        openingExercise,
        resourceId: exercise.resource_id,
      })
      return
    }
    openingExercise = true

    void logExerciseEvent('info', 'open start', {
      productCode: currentBook.value.product_code,
      resourceId: exercise.resource_id,
      title: exercise.name,
    })

    const loadingToken = appStore.startGlobalLoading(t('reader.loadingExercise'))
    appStore.setGlobalLoadingProgress(loadingToken, 0)
    appStore.setGlobalLoadingMessage(loadingToken, `${t('reader.loadingExerciseDeps')} (0/0)`)
    let unlistenDownloadProgress: UnlistenFn | null = null

    try {
      unlistenDownloadProgress = await listen<ExerciseDownloadProgressEvent>(
        'exercise-download-progress',
        (event) => {
          const payload = event.payload
          if (!currentBook.value || payload.productCode !== currentBook.value.product_code) return
          const isDepsStage = payload.stage === 'deps'
          if (!isDepsStage && payload.resourceId && payload.resourceId !== exercise.resource_id)
            return

          void logExerciseEvent('info', 'download progress', {
            resourceId: exercise.resource_id,
            stage: payload.stage,
            completedFiles: payload.completedFiles,
            totalFiles: payload.totalFiles,
            percent: payload.percent,
            done: payload.done,
          })

          appStore.setGlobalLoadingProgress(loadingToken, payload.percent)
          const stageText =
            payload.stage === 'deps'
              ? t('reader.loadingExerciseDeps')
              : t('reader.loadingExercisePackage')
          appStore.setGlobalLoadingMessage(
            loadingToken,
            `${stageText} (${payload.completedFiles}/${payload.totalFiles})`,
          )
        },
      )

      exerciseVisible.value = false
      currentExerciseHtml.value = ''
      currentExerciseUrl.value = ''
      currentExerciseResourceId.value = ''

      const res = await getExerciseHtml(currentBook.value.product_code, exercise.resource_id)
      void logExerciseEvent('info', 'html loaded', {
        resourceId: exercise.resource_id,
        htmlLength: res.html.length,
        url: res.url,
        preview: res.html.slice(0, 160),
      })
      const runtimePaths = extractExerciseRuntimePaths(res.html)
      void logExerciseEvent('info', 'html ready', {
        resourceId: exercise.resource_id,
        length: res.html.length,
        hasEnginePath: Boolean(runtimePaths.engine),
        hasDpPath: Boolean(runtimePaths.dp),
        enginePathPrefix: runtimePaths.engine ? runtimePaths.engine.slice(0, 80) : '',
      })

      currentExerciseHtml.value = res.html
      currentExerciseUrl.value = res.url
      currentExerciseTitle.value = exercise.name
      currentExerciseResourceId.value = exercise.resource_id
      exerciseVisible.value = true
      void logExerciseEvent('info', 'open visible', {
        resourceId: exercise.resource_id,
        visible: exerciseVisible.value,
      })
    } catch (error) {
      const errorMessage = getReadableCommandError(error)
      void logExerciseEvent('error', 'open failed', {
        resourceId: exercise.resource_id,
        error: errorMessage,
      })
      antMessage.error(errorMessage)
      console.error('Failed to resolve exercise:', error)
    } finally {
      if (unlistenDownloadProgress) {
        unlistenDownloadProgress()
        unlistenDownloadProgress = null
      }
      appStore.stopGlobalLoading(loadingToken)
      openingExercise = false
      void logExerciseEvent('info', 'open finished', {
        resourceId: exercise.resource_id,
        visible: exerciseVisible.value,
      })
    }
  }

  return {
    openExercise,
  }
}
