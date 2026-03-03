import { computed, type Ref } from 'vue'
import type { BookMetadata, OverlayAudio, TocNode } from '@/types'

interface UseReaderTocContextOptions {
  metadata: Ref<BookMetadata | null>
  currentPageLabel: Ref<string>
  leftPageLabel: Ref<string>
  rightPageLabel: Ref<string>
  viewMode: Ref<'single' | 'spread'>
  sortedPageLabels: Ref<string[]>
  fallbackUnitTitle: Ref<string>
}

function findDeepestTitleForPage(
  nodes: TocNode[],
  pageIndex: number,
  sortedPageLabels: string[],
): string | null {
  for (const node of nodes) {
    if (node.startPage && node.endPage) {
      const startIndex = sortedPageLabels.indexOf(node.startPage)
      const endIndex = sortedPageLabels.indexOf(node.endPage)
      if (
        startIndex !== -1 &&
        endIndex !== -1 &&
        pageIndex >= startIndex &&
        pageIndex <= endIndex
      ) {
        if (node.children?.length) {
          const childTitle = findDeepestTitleForPage(node.children, pageIndex, sortedPageLabels)
          if (childTitle) return childTitle
        }
        return node.title
      }
    } else if (node.children?.length) {
      const childTitle = findDeepestTitleForPage(node.children, pageIndex, sortedPageLabels)
      if (childTitle) return childTitle
    }
  }
  return null
}

function findBestAudioNodeForPage(
  nodes: TocNode[],
  pageIndex: number,
  sortedPageLabels: string[],
): TocNode | null {
  let found: TocNode | null = null

  for (const node of nodes) {
    if (node.startPage && node.endPage) {
      const startIndex = sortedPageLabels.indexOf(node.startPage)
      const endIndex = sortedPageLabels.indexOf(node.endPage)
      if (
        startIndex !== -1 &&
        endIndex !== -1 &&
        pageIndex >= startIndex &&
        pageIndex <= endIndex
      ) {
        if (node.audioFiles?.length) {
          found = node
        }
        if (node.children?.length) {
          const childMatch = findBestAudioNodeForPage(node.children, pageIndex, sortedPageLabels)
          if (childMatch) {
            found = childMatch
          }
        }
        if (found) break
      }
    } else if (node.children?.length) {
      const childMatch = findBestAudioNodeForPage(node.children, pageIndex, sortedPageLabels)
      if (childMatch) return childMatch
    }
  }

  return found
}

export function useReaderTocContext({
  metadata,
  currentPageLabel,
  leftPageLabel,
  rightPageLabel,
  viewMode,
  sortedPageLabels,
  fallbackUnitTitle,
}: UseReaderTocContextOptions) {
  const currentUnitName = computed(() => {
    if (!metadata.value || !currentPageLabel.value) return ''

    const pageIndex = sortedPageLabels.value.indexOf(leftPageLabel.value)
    const title = findDeepestTitleForPage(metadata.value.toc, pageIndex, sortedPageLabels.value)
    return title || fallbackUnitTitle.value || ''
  })

  const currentPageAudioFiles = computed<OverlayAudio[]>(() => {
    if (!metadata.value || !currentPageLabel.value) return []

    const labelsToCheck = [leftPageLabel.value]
    if (viewMode.value === 'spread' && rightPageLabel.value) {
      labelsToCheck.push(rightPageLabel.value)
    }

    for (let i = labelsToCheck.length - 1; i >= 0; i--) {
      const pageIndex = sortedPageLabels.value.indexOf(labelsToCheck[i])
      if (pageIndex === -1) continue

      const node = findBestAudioNodeForPage(metadata.value.toc, pageIndex, sortedPageLabels.value)
      if (node?.audioFiles?.length) return node.audioFiles
    }

    return []
  })

  return {
    currentUnitName,
    currentPageAudioFiles,
  }
}
