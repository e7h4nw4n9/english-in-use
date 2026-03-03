import { computed, type Ref } from 'vue'

interface UseReaderPaginationOptions {
  pageLabels: Ref<string[]>
  currentPageLabel: Ref<string>
  viewMode: Ref<'single' | 'spread'>
  spreadOffset: Ref<number>
}

export function useReaderPagination({
  pageLabels,
  currentPageLabel,
  viewMode,
  spreadOffset,
}: UseReaderPaginationOptions) {
  const currentIndex = computed(() => {
    return pageLabels.value.indexOf(currentPageLabel.value)
  })

  const displayIndex = computed(() => {
    if (viewMode.value === 'single') return currentIndex.value

    const idx = currentIndex.value
    if (idx === -1) return 0

    const label = pageLabels.value[idx]
    const pageNum = parseInt(label)

    if (!isNaN(pageNum)) {
      if (pageNum % 2 === 0) {
        return idx
      }
      if (idx > 0) {
        const prevLabel = pageLabels.value[idx - 1]
        const prevNum = parseInt(prevLabel)
        if (!isNaN(prevNum) && prevNum === pageNum - 1) {
          return idx - 1
        }
      }
      return idx
    }

    const offset = spreadOffset.value
    const base = Math.floor((idx - offset) / 2) * 2 + offset
    return Math.max(0, base)
  })

  const leftPageLabel = computed(() => {
    if (viewMode.value === 'single') return currentPageLabel.value

    const idx = displayIndex.value
    const label = pageLabels.value[idx]
    const pageNum = parseInt(label)

    if (!isNaN(pageNum) && pageNum % 2 !== 0) {
      return ''
    }

    return label || ''
  })

  const rightPageLabel = computed(() => {
    if (viewMode.value === 'single') return ''

    const idx = displayIndex.value
    const leftLabel = pageLabels.value[idx]
    const leftNum = parseInt(leftLabel)

    if (!isNaN(leftNum) && leftNum % 2 !== 0) {
      return leftLabel
    }

    return pageLabels.value[idx + 1] || ''
  })

  const canGoBack = computed(() => displayIndex.value > 0)

  const canGoForward = computed(() => {
    const step = viewMode.value === 'spread' ? 2 : 1
    return displayIndex.value + step < pageLabels.value.length
  })

  function goBack() {
    if (!canGoBack.value) return
    const step = viewMode.value === 'spread' ? 2 : 1
    const newIndex = Math.max(0, displayIndex.value - step)
    currentPageLabel.value = pageLabels.value[newIndex]
  }

  function goForward() {
    if (!canGoForward.value) return
    const step = viewMode.value === 'spread' ? 2 : 1
    const newIndex = Math.min(pageLabels.value.length - 1, displayIndex.value + step)
    currentPageLabel.value = pageLabels.value[newIndex]
  }

  return {
    currentIndex,
    displayIndex,
    leftPageLabel,
    rightPageLabel,
    canGoBack,
    canGoForward,
    goBack,
    goForward,
  }
}
