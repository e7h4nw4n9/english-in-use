import { ref, computed, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '../stores/app'
import { useReaderStore } from '../stores/reader'
import { getBookMetadata, resolvePageResource } from '../lib/api/books'
import { getReadableCommandError } from '../lib/error'
import type { BookMetadata } from '../types'
import { useReaderPagination } from './reader/useReaderPagination'
import { useReaderProgress } from './reader/useReaderProgress'
import { useReaderPreload } from './reader/useReaderPreload'

export function useReaderMetadata() {
  const appStore = useAppStore()
  const readerStore = useReaderStore()
  const { currentBook } = storeToRefs(appStore)
  const { viewMode, zoomLevel, currentPageLabel, spreadOffset } = storeToRefs(readerStore)

  const metadata = ref<BookMetadata | null>(null)
  const loading = ref(true)
  const leftPageUrl = ref<string>('')
  const rightPageUrl = ref<string>('')

  const sortedPageLabels = computed(() => {
    if (!metadata.value) return []
    if (metadata.value.pageLabels) return metadata.value.pageLabels
    return Object.keys(metadata.value.pages).sort((a, b) => {
      const na = parseInt(a)
      const nb = parseInt(b)
      if (!isNaN(na) && !isNaN(nb)) return na - nb
      return a.localeCompare(b)
    })
  })

  const {
    currentIndex,
    displayIndex,
    leftPageLabel,
    rightPageLabel,
    canGoBack,
    canGoForward,
    goBack,
    goForward,
  } = useReaderPagination({
    pageLabels: sortedPageLabels,
    currentPageLabel,
    viewMode,
    spreadOffset,
  })

  const { restoreProgress, saveProgress } = useReaderProgress({
    currentBook,
    metadata,
    currentPageLabel,
    zoomLevel,
    sortedPageLabels,
  })

  const { resetPreload, triggerPreload } = useReaderPreload({
    currentBook,
    metadata,
    displayIndex,
    viewMode,
    sortedPageLabels,
  })

  async function loadMetadata() {
    if (!currentBook.value) return

    loading.value = true
    resetPreload()

    try {
      metadata.value = await getBookMetadata(currentBook.value.product_code)
      await restoreProgress()
      await updatePageUrls()
    } catch (e) {
      console.error('Failed to load metadata:', getReadableCommandError(e))
    } finally {
      loading.value = false
    }
  }

  async function updatePageUrls() {
    if (!currentBook.value || !metadata.value) return

    try {
      leftPageUrl.value = await resolvePageResource(
        currentBook.value.product_code,
        leftPageLabel.value,
      )

      if (viewMode.value === 'spread' && rightPageLabel.value) {
        rightPageUrl.value = await resolvePageResource(
          currentBook.value.product_code,
          rightPageLabel.value,
        )
      } else {
        rightPageUrl.value = ''
      }
    } catch (e) {
      console.error('Failed to resolve page resource:', getReadableCommandError(e))
    }
  }

  watch(currentPageLabel, () => {
    updatePageUrls()
    saveProgress()
    triggerPreload()
  })

  watch(viewMode, updatePageUrls)
  watch(zoomLevel, saveProgress)

  return {
    metadata,
    loading,
    leftPageUrl,
    rightPageUrl,
    sortedPageLabels,
    currentIndex,
    displayIndex,
    leftPageLabel,
    rightPageLabel,
    canGoBack,
    canGoForward,
    loadMetadata,
    saveProgress,
    updatePageUrls,
    goBack,
    goForward,
  }
}
