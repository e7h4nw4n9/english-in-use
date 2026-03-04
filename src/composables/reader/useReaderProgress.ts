import type { Ref } from 'vue'
import type { Book, BookMetadata } from '@/types'
import { getReadingProgress, updateReadingProgress } from '@/lib/api/books'
import { useReaderStore } from '@/stores/reader'

interface UseReaderProgressOptions {
  currentBook: Ref<Book | null>
  metadata: Ref<BookMetadata | null>
  currentPageLabel: Ref<string>
  zoomLevel: Ref<number>
  sortedPageLabels: Ref<string[]>
}

export function useReaderProgress({
  currentBook,
  metadata,
  currentPageLabel,
  zoomLevel,
  sortedPageLabels,
}: UseReaderProgressOptions) {
  const readerStore = useReaderStore()

  function resolvePageLabelByResourceId(resourceId: string): string | null {
    if (!metadata.value || !resourceId) return null

    for (const label of sortedPageLabels.value) {
      if (metadata.value.pages[label]?.resource_id === resourceId) {
        return label
      }
    }
    return null
  }

  async function restoreProgress() {
    if (!currentBook.value) return

    try {
      if (readerStore.pendingStudyResourceId) {
        const pendingTarget = resolvePageLabelByResourceId(readerStore.pendingStudyResourceId)
        readerStore.pendingStudyResourceId = null
        if (pendingTarget) {
          currentPageLabel.value = pendingTarget
          return
        }
      }

      const progress = await getReadingProgress(currentBook.value.product_code)
      if (!progress) return

      if (progress.page_label && sortedPageLabels.value.includes(progress.page_label)) {
        currentPageLabel.value = progress.page_label
      } else if (progress.resource_id) {
        const targetLabel = resolvePageLabelByResourceId(progress.resource_id)
        if (targetLabel) {
          currentPageLabel.value = targetLabel
        }
      }

      if (progress.scale) {
        zoomLevel.value = progress.scale
      }
    } catch (e) {
      console.error('Failed to restore progress:', e)
    }
  }

  async function saveProgress() {
    if (!currentBook.value) return

    try {
      const resourceId = metadata.value?.pages[currentPageLabel.value]?.resource_id || null
      await updateReadingProgress(
        currentBook.value.product_code,
        resourceId,
        currentPageLabel.value,
        zoomLevel.value,
        0,
        0,
      )
    } catch (e) {
      console.error('Failed to save progress:', e)
    }
  }

  return {
    restoreProgress,
    saveProgress,
  }
}
