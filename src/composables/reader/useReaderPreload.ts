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

export function useReaderPreload({
  currentBook,
  metadata,
  displayIndex,
  viewMode,
  sortedPageLabels,
}: UseReaderPreloadOptions) {
  const preloadedSet = ref(new Set<string>())
  let preloadTimeout: ReturnType<typeof setTimeout> | null = null

  function resetPreload() {
    preloadedSet.value.clear()
    if (preloadTimeout) {
      clearTimeout(preloadTimeout)
      preloadTimeout = null
    }
  }

  function triggerPreload() {
    if (preloadTimeout) clearTimeout(preloadTimeout)

    preloadTimeout = setTimeout(async () => {
      if (!metadata.value || !currentBook.value) return

      const step = viewMode.value === 'spread' ? 2 : 1
      const targets: number[] = []

      for (let i = 1; i <= 2; i++) {
        const idx = displayIndex.value + i * step
        if (idx < sortedPageLabels.value.length) targets.push(idx)
        if (viewMode.value === 'spread' && idx + 1 < sortedPageLabels.value.length) {
          targets.push(idx + 1)
        }
      }

      const backwardIndex = displayIndex.value - step
      if (backwardIndex >= 0) {
        targets.push(backwardIndex)
        if (viewMode.value === 'spread' && backwardIndex + 1 < sortedPageLabels.value.length) {
          targets.push(backwardIndex + 1)
        }
      }

      for (const idx of targets) {
        const label = sortedPageLabels.value[idx]
        if (!preloadedSet.value.has(label)) {
          try {
            const url = await resolvePageResource(currentBook.value.product_code, label)
            const img = new Image()
            img.src = url
            preloadedSet.value.add(label)
          } catch (e) {
            // ignore preload miss
          }
        }
      }

      const audioTargets = [displayIndex.value]
      if (viewMode.value === 'spread') audioTargets.push(displayIndex.value + 1)

      const audioPaths = new Set<string>()
      for (const idx of audioTargets) {
        const label = sortedPageLabels.value[idx]
        const pageOverlays = metadata.value.pages[label]?.overlays || []
        pageOverlays.forEach((overlay) => {
          if (overlay.type === 'audio' && overlay.audio) {
            audioPaths.add(overlay.audio.path)
          }
        })
      }

      for (const path of audioPaths) {
        try {
          await resolveBookAsset(currentBook.value.product_code, path)
        } catch (e) {
          // ignore preload miss
        }
      }
    }, 1000)
  }

  onUnmounted(() => {
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
