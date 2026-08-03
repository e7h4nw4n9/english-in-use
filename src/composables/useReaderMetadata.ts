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

/** 管理当前图书元数据的加载状态与页面索引。 */
export function useReaderMetadata() {
  const appStore = useAppStore()
  const readerStore = useReaderStore()
  const { currentBook } = storeToRefs(appStore)
  const { viewMode, zoomLevel, currentPageLabel, spreadOffset } = storeToRefs(readerStore)

  const metadata = ref<BookMetadata | null>(null)
  const loading = ref(true)
  const leftPageUrl = ref<string>('')
  const rightPageUrl = ref<string>('')
  let metadataRequestVersion = 0
  let pageUrlRequestVersion = 0

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

  /** 加载书籍元数据并在同一请求版本内恢复进度。 */
  async function loadMetadata() {
    if (!currentBook.value) return
    const productCode = currentBook.value.product_code
    const requestVersion = ++metadataRequestVersion

    loading.value = true
    resetPreload()

    try {
      const loadedMetadata = await getBookMetadata(productCode)
      if (
        requestVersion !== metadataRequestVersion ||
        currentBook.value?.product_code !== productCode
      ) {
        return
      }
      metadata.value = loadedMetadata
      await restoreProgress()
      if (requestVersion !== metadataRequestVersion) return
      if (!sortedPageLabels.value.includes(currentPageLabel.value)) {
        currentPageLabel.value = sortedPageLabels.value[0] ?? ''
      }
      await updatePageUrls()
      triggerPreload()
    } catch (e) {
      console.error('Failed to load metadata:', getReadableCommandError(e))
    } finally {
      if (requestVersion === metadataRequestVersion) {
        loading.value = false
      }
    }
  }

  /** 解析当前视图需要展示的页面资源。 */
  async function updatePageUrls() {
    if (!currentBook.value || !metadata.value) return
    const requestVersion = ++pageUrlRequestVersion
    const productCode = currentBook.value.product_code
    const leftLabel = leftPageLabel.value
    const rightLabel = viewMode.value === 'spread' ? rightPageLabel.value : null
    leftPageUrl.value = ''
    rightPageUrl.value = ''

    try {
      const [nextLeftUrl, nextRightUrl] = await Promise.all([
        leftLabel ? resolvePageResource(productCode, leftLabel) : Promise.resolve(''),
        rightLabel ? resolvePageResource(productCode, rightLabel) : Promise.resolve(''),
      ])
      if (
        requestVersion !== pageUrlRequestVersion ||
        currentBook.value?.product_code !== productCode
      ) {
        return
      }
      leftPageUrl.value = nextLeftUrl
      rightPageUrl.value = nextRightUrl
    } catch (e) {
      console.error('Failed to resolve page resource:', getReadableCommandError(e))
    }
  }

  watch(currentPageLabel, () => {
    updatePageUrls()
    saveProgress()
    triggerPreload()
  })

  watch(viewMode, () => {
    updatePageUrls()
    triggerPreload()
  })
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
