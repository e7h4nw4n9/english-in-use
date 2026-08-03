import { onUnmounted, ref, type Ref } from 'vue'
import { resolveBookAsset, resolvePageResource } from '@/lib/api/books'
import type { Book, BookMetadata } from '@/types'

interface UseReaderPreloadOptions {
  currentBook: Ref<Book | null>
  metadata: Ref<BookMetadata | null>
  displayIndex: Ref<number>
  viewMode: Ref<'single' | 'spread'>
  sortedPageLabels: Ref<string[]>
}

/**
 * 预加载相邻页面图片和当前跨页音频资源。
 * @param options - 当前图书、元数据、视图模式和页面索引引用。
 */
export function useReaderPreload({
  currentBook,
  metadata,
  displayIndex,
  viewMode,
  sortedPageLabels,
}: UseReaderPreloadOptions) {
  const preloadedSet = ref(new Set<string>())
  const inFlightGenerations = new Map<string, number>()
  let preloadTimeout: ReturnType<typeof setTimeout> | null = null
  let preloadGeneration = 0
  let activePreloadTasks = 0
  const pendingPreloadTasks: Array<() => Promise<void>> = []

  /** 清理当前书籍的预加载状态，并使已启动的旧任务失效。 */
  function resetPreload() {
    preloadGeneration++
    preloadedSet.value.clear()
    inFlightGenerations.clear()
    pendingPreloadTasks.length = 0
    if (preloadTimeout) {
      clearTimeout(preloadTimeout)
      preloadTimeout = null
    }
  }

  /** 启动队列中的预加载任务，并将全局并发限制为两个。 */
  function pumpPreloadQueue() {
    while (activePreloadTasks < 2) {
      const task = pendingPreloadTasks.shift()
      if (!task) return
      activePreloadTasks++
      void task().finally(() => {
        activePreloadTasks--
        pumpPreloadQueue()
      })
    }
  }

  /** 将当前批次加入共享预加载队列。 */
  function enqueuePreloadTasks(tasks: Array<() => Promise<void>>) {
    pendingPreloadTasks.push(...tasks)
    pumpPreloadQueue()
  }

  /** 延迟预加载相邻页面与当前页音频。 */
  function triggerPreload() {
    if (preloadTimeout) clearTimeout(preloadTimeout)

    preloadTimeout = setTimeout(async () => {
      if (!metadata.value || !currentBook.value) return
      const generation = preloadGeneration
      const productCode = currentBook.value.product_code
      const metadataSnapshot = metadata.value
      const labels = [...sortedPageLabels.value]
      const currentDisplayIndex = displayIndex.value
      const currentViewMode = viewMode.value

      const step = currentViewMode === 'spread' ? 2 : 1
      const targets: number[] = []

      for (let i = 1; i <= 2; i++) {
        const idx = currentDisplayIndex + i * step
        if (idx < labels.length) targets.push(idx)
        if (currentViewMode === 'spread' && idx + 1 < labels.length) {
          targets.push(idx + 1)
        }
      }

      const backwardIndex = currentDisplayIndex - step
      if (backwardIndex >= 0) {
        targets.push(backwardIndex)
        if (currentViewMode === 'spread' && backwardIndex + 1 < labels.length) {
          targets.push(backwardIndex + 1)
        }
      }

      const tasks: Array<() => Promise<void>> = []
      for (const idx of targets) {
        const label = labels[idx]
        const preloadKey = `${productCode}:${label}`
        if (!preloadedSet.value.has(preloadKey) && !inFlightGenerations.has(preloadKey)) {
          inFlightGenerations.set(preloadKey, generation)
          tasks.push(async () => {
            try {
              const url = await resolvePageResource(productCode, label)
              if (
                generation !== preloadGeneration ||
                currentBook.value?.product_code !== productCode
              ) {
                return
              }
              const img = new Image()
              img.src = url
              preloadedSet.value.add(preloadKey)
            } catch {
              // 预加载失败不影响当前页面展示。
            } finally {
              if (inFlightGenerations.get(preloadKey) === generation) {
                inFlightGenerations.delete(preloadKey)
              }
            }
          })
        }
      }

      const audioTargets = [currentDisplayIndex]
      if (currentViewMode === 'spread') audioTargets.push(currentDisplayIndex + 1)

      const audioPaths = new Set<string>()
      for (const idx of audioTargets) {
        const label = labels[idx]
        const pageOverlays = metadataSnapshot.pages[label]?.overlays || []
        pageOverlays.forEach((overlay) => {
          if (overlay.type === 'audio' && overlay.audio) {
            audioPaths.add(overlay.audio.path)
          }
        })
      }

      for (const path of audioPaths) {
        const preloadKey = `${productCode}:asset:${path}`
        if (!preloadedSet.value.has(preloadKey) && !inFlightGenerations.has(preloadKey)) {
          inFlightGenerations.set(preloadKey, generation)
          tasks.push(async () => {
            try {
              await resolveBookAsset(productCode, path)
              if (
                generation !== preloadGeneration ||
                currentBook.value?.product_code !== productCode
              ) {
                return
              }
              preloadedSet.value.add(preloadKey)
            } catch {
              // 预加载失败不影响当前音频播放。
            } finally {
              if (inFlightGenerations.get(preloadKey) === generation) {
                inFlightGenerations.delete(preloadKey)
              }
            }
          })
        }
      }

      enqueuePreloadTasks(tasks)
    }, 1000)
  }

  onUnmounted(() => {
    preloadGeneration++
    inFlightGenerations.clear()
    pendingPreloadTasks.length = 0
    if (preloadTimeout) {
      clearTimeout(preloadTimeout)
      preloadTimeout = null
    }
  })

  return {
    resetPreload,
    triggerPreload,
  }
}
