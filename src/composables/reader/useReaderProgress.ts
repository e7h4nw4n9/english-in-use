import { onBeforeUnmount, type Ref } from 'vue'
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

interface ProgressSnapshot {
  productCode: string
  resourceId: string | null
  pageLabel: string
  scale: number
}

/** 判断两个保存快照是否完全一致。 */
function isSameSnapshot(left: ProgressSnapshot | undefined, right: ProgressSnapshot): boolean {
  return (
    left?.productCode === right.productCode &&
    left.resourceId === right.resourceId &&
    left.pageLabel === right.pageLabel &&
    left.scale === right.scale
  )
}

/**
 * 恢复并串行保存阅读进度，避免旧请求覆盖新状态。
 * @param options - 当前图书、页面、缩放和元数据引用。
 */
export function useReaderProgress({
  currentBook,
  metadata,
  currentPageLabel,
  zoomLevel,
  sortedPageLabels,
}: UseReaderProgressOptions) {
  const readerStore = useReaderStore()
  let restoreRequestVersion = 0
  let restoringProgress = false
  let saveTimer: ReturnType<typeof setTimeout> | null = null
  let saveInFlight: Promise<void> | null = null
  let pendingSave: ProgressSnapshot | undefined
  let lastPersistedSave: ProgressSnapshot | undefined

  /** 根据资源标识查找对应页码。 */
  function resolvePageLabelByResourceId(resourceId: string): string | null {
    if (!metadata.value || !resourceId) return null

    for (const label of sortedPageLabels.value) {
      if (metadata.value.pages[label]?.resource_id === resourceId) {
        return label
      }
    }
    return null
  }

  /** 恢复当前书籍的阅读进度，恢复期间不触发反向保存。 */
  async function restoreProgress() {
    if (!currentBook.value) return
    const productCode = currentBook.value.product_code
    const requestVersion = ++restoreRequestVersion
    restoringProgress = true

    try {
      if (readerStore.pendingStudyResourceId) {
        const pendingTarget = resolvePageLabelByResourceId(readerStore.pendingStudyResourceId)
        readerStore.pendingStudyResourceId = null
        if (pendingTarget) {
          currentPageLabel.value = pendingTarget
          return
        }
      }

      const progress = await getReadingProgress(productCode)
      if (
        requestVersion !== restoreRequestVersion ||
        currentBook.value?.product_code !== productCode
      ) {
        return
      }
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
    } finally {
      if (requestVersion === restoreRequestVersion) {
        restoringProgress = false
      }
    }
  }

  /** 对当前阅读状态生成防抖保存快照。 */
  function saveProgress() {
    if (!currentBook.value || restoringProgress) return

    const snapshot = {
      productCode: currentBook.value.product_code,
      resourceId: metadata.value?.pages[currentPageLabel.value]?.resource_id || null,
      pageLabel: currentPageLabel.value,
      scale: zoomLevel.value,
    }
    if (isSameSnapshot(lastPersistedSave, snapshot) || isSameSnapshot(pendingSave, snapshot)) return
    pendingSave = snapshot
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => {
      saveTimer = null
      void flushProgress()
    }, 300)
  }

  /** 串行写入保存快照，避免较慢的旧请求覆盖较新的进度。 */
  async function flushProgress() {
    if (saveTimer) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
    if (saveInFlight) {
      await saveInFlight
      return flushProgress()
    }

    const snapshot = pendingSave
    pendingSave = undefined
    if (!snapshot) return

    const request = updateReadingProgress(
      snapshot.productCode,
      snapshot.resourceId,
      snapshot.pageLabel,
      snapshot.scale,
      0,
      0,
    )
      .then(() => {
        lastPersistedSave = snapshot
      })
      .catch((e) => {
        console.error('Failed to save progress:', e)
      })
    saveInFlight = request
    await request
    if (saveInFlight === request) {
      saveInFlight = null
    }
    if (pendingSave) {
      await flushProgress()
    }
  }

  onBeforeUnmount(() => {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = null
    void flushProgress()
  })

  return {
    restoreProgress,
    saveProgress,
    flushProgress,
  }
}
