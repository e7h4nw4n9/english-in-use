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

function resolvePageRange(
  node: TocNode,
  sortedPageLabels: string[],
): { startIndex: number; endIndex: number } | null {
  if (!node.startPage || !node.endPage) return null
  const startIndex = sortedPageLabels.indexOf(node.startPage)
  const endIndex = sortedPageLabels.indexOf(node.endPage)
  if (startIndex === -1 || endIndex === -1 || endIndex < startIndex) {
    return null
  }
  return { startIndex, endIndex }
}

function getPageSpan(node: TocNode, sortedPageLabels: string[]): number {
  const range = resolvePageRange(node, sortedPageLabels)
  if (!range) return 0
  return range.endIndex - range.startIndex + 1
}

function findDeepestPathForPage(
  nodes: TocNode[],
  pageIndex: number,
  sortedPageLabels: string[],
): TocNode[] | null {
  for (const node of nodes) {
    const range = resolvePageRange(node, sortedPageLabels)
    const inRange = range !== null && pageIndex >= range.startIndex && pageIndex <= range.endIndex

    if (inRange) {
      if (node.children?.length) {
        const childPath = findDeepestPathForPage(node.children, pageIndex, sortedPageLabels)
        if (childPath) return [node, ...childPath]
      }
      return [node]
    }

    if (!range && node.children?.length) {
      const childPath = findDeepestPathForPage(node.children, pageIndex, sortedPageLabels)
      if (childPath) return [node, ...childPath]
    }
  }
  return null
}

function selectStudyPlanAnchor(path: TocNode[], sortedPageLabels: string[]): TocNode | null {
  if (!path.length) return null

  const deepest = path[path.length - 1]
  if (getPageSpan(deepest, sortedPageLabels) >= 2) {
    return deepest
  }

  for (let i = path.length - 2; i >= 0; i -= 1) {
    if (getPageSpan(path[i], sortedPageLabels) >= 2) {
      return path[i]
    }
  }

  return deepest
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
  const currentPageIndex = computed(() => sortedPageLabels.value.indexOf(leftPageLabel.value))

  const currentTocPath = computed<TocNode[] | null>(() => {
    if (!metadata.value || !currentPageLabel.value) return null
    if (currentPageIndex.value === -1) return null

    return findDeepestPathForPage(
      metadata.value.toc,
      currentPageIndex.value,
      sortedPageLabels.value,
    )
  })

  const currentStudyPlanAnchor = computed<TocNode | null>(() => {
    const path = currentTocPath.value
    if (!path?.length) return null
    return selectStudyPlanAnchor(path, sortedPageLabels.value)
  })

  const currentUnitName = computed(() => {
    if (!metadata.value || !currentPageLabel.value) return ''

    const title = findDeepestTitleForPage(
      metadata.value.toc,
      currentPageIndex.value,
      sortedPageLabels.value,
    )
    return title || fallbackUnitTitle.value || ''
  })

  const currentStudyPlanUnitName = computed(() => {
    const anchor = currentStudyPlanAnchor.value
    if (anchor?.title) return anchor.title
    return currentUnitName.value || fallbackUnitTitle.value || ''
  })

  const currentStudyPlanResourceId = computed(() => {
    if (!metadata.value || !currentPageLabel.value) return null

    const anchorStartPage = currentStudyPlanAnchor.value?.startPage
    if (anchorStartPage) {
      const anchorResourceId = metadata.value.pages[anchorStartPage]?.resource_id || null
      if (anchorResourceId) return anchorResourceId
    }

    return metadata.value.pages[currentPageLabel.value]?.resource_id || null
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
    currentStudyPlanUnitName,
    currentStudyPlanResourceId,
    currentPageAudioFiles,
  }
}
