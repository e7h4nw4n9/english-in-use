import { ref, computed, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '../stores/app'
import { useReaderStore } from '../stores/reader'
import {
  getBookMetadata,
  resolvePageResource,
  getReadingProgress,
  updateReadingProgress,
  resolveBookAsset,
} from '../lib/api/books'
import type { BookMetadata } from '../types'

export function useReaderMetadata() {
  const appStore = useAppStore()
  const readerStore = useReaderStore()
  const { currentBook } = storeToRefs(appStore)
  const { viewMode, zoomLevel, currentPageLabel, spreadOffset } = storeToRefs(readerStore)

  const metadata = ref<BookMetadata | null>(null)
  const loading = ref(true)
  const leftPageUrl = ref<string>('')
  const rightPageUrl = ref<string>('')
  const preloadedSet = ref(new Set<string>())
  let preloadTimeout: any = null

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

  const currentIndex = computed(() => {
    return sortedPageLabels.value.indexOf(currentPageLabel.value)
  })

  const displayIndex = computed(() => {
    if (viewMode.value === 'single') return currentIndex.value

    const idx = currentIndex.value
    if (idx === -1) return 0

    const label = sortedPageLabels.value[idx]
    const pageNum = parseInt(label)

    // If page label is a number, align by parity (Even on Left, Odd on Right)
    if (!isNaN(pageNum)) {
      if (pageNum % 2 === 0) {
        // Even page is the left page
        return idx
      } else {
        // Odd page is the right page, so the left page is the previous one
        // but only if the previous one exists and is the expected even number
        if (idx > 0) {
          const prevLabel = sortedPageLabels.value[idx - 1]
          const prevNum = parseInt(prevLabel)
          if (!isNaN(prevNum) && prevNum === pageNum - 1) {
            return idx - 1
          }
        }
        // If no even page exists before it (e.g. Page 1),
        // we return idx but we'll handle the empty left page in leftPageLabel
        return idx
      }
    }

    // Fallback to index-based parity if not a number
    const offset = spreadOffset.value
    const base = Math.floor((idx - offset) / 2) * 2 + offset
    return Math.max(0, base)
  })

  const leftPageLabel = computed(() => {
    if (viewMode.value === 'single') return currentPageLabel.value

    const idx = displayIndex.value
    const label = sortedPageLabels.value[idx]
    const pageNum = parseInt(label)

    // In spread mode, if we are at an odd page index that should be on the right,
    // the left page is empty.
    if (!isNaN(pageNum) && pageNum % 2 !== 0) {
      return ''
    }

    return label || ''
  })

  const rightPageLabel = computed(() => {
    if (viewMode.value === 'single') return ''

    const idx = displayIndex.value
    const leftLabel = sortedPageLabels.value[idx]
    const leftNum = parseInt(leftLabel)

    // If the 'left' index actually points to an odd page (meaning it's the first page of the book),
    // then that page is actually the right page.
    if (!isNaN(leftNum) && leftNum % 2 !== 0) {
      return leftLabel
    }

    return sortedPageLabels.value[idx + 1] || ''
  })

  const canGoBack = computed(() => displayIndex.value > 0)
  const canGoForward = computed(() => {
    const step = viewMode.value === 'spread' ? 2 : 1
    return displayIndex.value + step < sortedPageLabels.value.length
  })

  async function loadMetadata() {
    if (!currentBook.value) return
    loading.value = true
    preloadedSet.value.clear()
    try {
      metadata.value = await getBookMetadata(currentBook.value.product_code)
      const progress = await getReadingProgress(currentBook.value.product_code)
      if (progress) {
        if (progress.page_label && sortedPageLabels.value.includes(progress.page_label)) {
          currentPageLabel.value = progress.page_label
        }
        if (progress.scale) {
          zoomLevel.value = progress.scale
        }
      }
      await updatePageUrls()
    } catch (e) {
      console.error('Failed to load metadata:', e)
    } finally {
      loading.value = false
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
      console.error('Failed to resolve page resource:', e)
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
        if (viewMode.value === 'spread' && idx + 1 < sortedPageLabels.value.length)
          targets.push(idx + 1)
      }
      const bIdx = displayIndex.value - step
      if (bIdx >= 0) {
        targets.push(bIdx)
        if (viewMode.value === 'spread' && bIdx + 1 < sortedPageLabels.value.length)
          targets.push(bIdx + 1)
      }
      for (const idx of targets) {
        const label = sortedPageLabels.value[idx]
        if (!preloadedSet.value.has(label)) {
          try {
            const url = await resolvePageResource(currentBook.value.product_code, label)
            const img = new Image()
            img.src = url
            preloadedSet.value.add(label)
          } catch (e) {}
        }
      }

      const audioTargets = [displayIndex.value]
      if (viewMode.value === 'spread') audioTargets.push(displayIndex.value + 1)

      const audioPaths = new Set<string>()
      for (const idx of audioTargets) {
        const label = sortedPageLabels.value[idx]
        const pageOverlays = metadata.value.pages[label]?.overlays || []
        pageOverlays.forEach((o) => {
          if (o.type === 'audio' && o.audio) {
            audioPaths.add(o.audio.path)
          }
        })
      }

      for (const path of audioPaths) {
        try {
          await resolveBookAsset(currentBook.value.product_code, path)
        } catch (e) {}
      }
    }, 1000)
  }

  function goBack() {
    if (!canGoBack.value) return
    const step = viewMode.value === 'spread' ? 2 : 1
    const newIndex = Math.max(0, displayIndex.value - step)
    currentPageLabel.value = sortedPageLabels.value[newIndex]
  }

  function goForward() {
    if (!canGoForward.value) return
    const step = viewMode.value === 'spread' ? 2 : 1
    const newIndex = Math.min(sortedPageLabels.value.length - 1, displayIndex.value + step)
    currentPageLabel.value = sortedPageLabels.value[newIndex]
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
