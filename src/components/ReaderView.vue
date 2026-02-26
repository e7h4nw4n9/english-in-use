<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watchEffect, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '../stores/app'
import { useReaderStore } from '../stores/reader'
import { useReaderAudio } from '../composables/useReaderAudio'
import { useReaderMetadata } from '../composables/useReaderMetadata'
import { resolveExerciseResource } from '../lib/api/books'
import type { TocNode, ExerciseInfo } from '../types'

// Components
import ReaderTOC from './reader/ReaderTOC.vue'
import ReaderCanvas from './reader/ReaderCanvas.vue'
import ReaderFooter from './reader/ReaderFooter.vue'
import ReaderAudioPlayer from './reader/ReaderAudioPlayer.vue'
import ReaderExerciseModal from './reader/ReaderExerciseModal.vue'
import ReaderDebugModal from './reader/ReaderDebugModal.vue'

const appStore = useAppStore()
const readerStore = useReaderStore()
const { currentBook } = storeToRefs(appStore)
const readerRef = ref<HTMLElement | null>(null)
const {
  currentPageLabel,
  viewMode,
  exerciseVisible,
  currentExerciseUrl,
  currentExerciseTitle,
  showHotspots,
  isUiVisible,
  isPlaying,
  currentAudioPath,
  isSidebarCollapsed,
} = storeToRefs(readerStore)

const {
  metadata,
  loading,
  leftPageUrl,
  rightPageUrl,
  leftPageLabel,
  rightPageLabel,
  sortedPageLabels,
  displayIndex,
  canGoBack,
  canGoForward,
  loadMetadata,
  goBack,
  goForward,
} = useReaderMetadata()

const { toggleAudio, pauseAudio, cleanup: audioCleanup } = useReaderAudio()

// Responsive View Mode
const isNarrow = ref(false)
const resizeObserver = new ResizeObserver((entries) => {
  for (const entry of entries) {
    const { width } = entry.contentRect
    isNarrow.value = width < 768
    if (isNarrow.value && viewMode.value === 'spread') {
      viewMode.value = 'single'
    }
  }
})

// Shortcuts
function handleKeyDown(e: KeyboardEvent) {
  // Navigation
  if (e.key === 'ArrowLeft') goBack()
  if (e.key === 'ArrowRight') goForward()
  if (e.key === ' ') {
    e.preventDefault()
    isPlaying.value = !isPlaying.value
  }
  if (e.key === 'Escape') closeReader()

  // Zoom
  if ((e.ctrlKey || e.metaKey) && (e.key === '=' || e.key === '+')) {
    e.preventDefault()
    readerStore.zoomIn()
  }
  if ((e.ctrlKey || e.metaKey) && e.key === '-') {
    e.preventDefault()
    readerStore.zoomOut()
  }
  if ((e.ctrlKey || e.metaKey) && e.key === '0') {
    e.preventDefault()
    readerStore.resetZoom()
  }
}

// Watch for unit name changes
watchEffect(() => {
  if (!metadata.value || !currentPageLabel.value) {
    readerStore.currentUnitName = ''
    return
  }

  const pageIdx = sortedPageLabels.value.indexOf(leftPageLabel.value)
  const search = (nodes: TocNode[]): string | null => {
    for (const node of nodes) {
      if (node.startPage && node.endPage) {
        const sIdx = sortedPageLabels.value.indexOf(node.startPage)
        const eIdx = sortedPageLabels.value.indexOf(node.endPage)
        if (sIdx !== -1 && eIdx !== -1 && pageIdx >= sIdx && pageIdx <= eIdx) {
          // Found matching node, but check children for more specific one
          if (node.children) {
            const childTitle = search(node.children)
            if (childTitle) return childTitle
          }
          return node.title
        }
      } else if (node.children) {
        const childTitle = search(node.children)
        if (childTitle) return childTitle
      }
    }
    return null
  }

  readerStore.currentUnitName = search(metadata.value.toc) || currentBook.value?.title || ''
})

// Computed logic for cross-component interactions
const currentPageExercises = computed(() => {
  const left = metadata.value?.pages[leftPageLabel.value]?.exercises || []
  const right =
    viewMode.value === 'spread' && rightPageLabel.value
      ? metadata.value?.pages[rightPageLabel.value]?.exercises || []
      : []
  return [...left, ...right]
})

const currentPageAudioFiles = computed(() => {
  if (!metadata.value || !currentPageLabel.value) return []
  const labelsToCheck = [leftPageLabel.value]
  if (viewMode.value === 'spread' && rightPageLabel.value) {
    labelsToCheck.push(rightPageLabel.value)
  }

  const findNodeForPage = (label: string): TocNode | null => {
    const pageIdx = sortedPageLabels.value.indexOf(label)
    if (pageIdx === -1) return null

    const search = (nodes: TocNode[]): TocNode | null => {
      let found: TocNode | null = null
      for (const node of nodes) {
        if (node.startPage && node.endPage) {
          const sIdx = sortedPageLabels.value.indexOf(node.startPage)
          const eIdx = sortedPageLabels.value.indexOf(node.endPage)
          if (sIdx !== -1 && eIdx !== -1 && pageIdx >= sIdx && pageIdx <= eIdx) {
            if (node.audioFiles?.length) found = node
            if (node.children) {
              const childMatch = search(node.children)
              if (childMatch) found = childMatch
            }
            if (found) break
          }
        } else if (node.children) {
          const childMatch = search(node.children)
          if (childMatch) return childMatch
        }
      }
      return found
    }
    return search(metadata.value!.toc)
  }

  for (let i = labelsToCheck.length - 1; i >= 0; i--) {
    const node = findNodeForPage(labelsToCheck[i])
    if (node?.audioFiles?.length) return node.audioFiles
  }
  return []
})

// Watch for page changes to stop audio if it's no longer on the current page
watch(currentPageAudioFiles, (newAudioFiles) => {
  if (isPlaying.value && currentAudioPath.value) {
    const isStillAvailable = newAudioFiles.some((file) => file.path === currentAudioPath.value)
    if (!isStillAvailable) {
      pauseAudio()
    }
  }
})

async function handleOverlayClick(overlay: any) {
  if (overlay.type === 'page' && overlay.page) {
    currentPageLabel.value = overlay.page.pagelabel
  } else if (overlay.type === 'audio' && overlay.audio) {
    if (currentBook.value) {
      await toggleAudio(currentBook.value.product_code, overlay.audio.path)
    }
  }
}

async function openExercise(ex: ExerciseInfo) {
  if (!currentBook.value) return
  try {
    const url = await resolveExerciseResource(currentBook.value.product_code, ex.resource_id)
    currentExerciseUrl.value = url
    currentExerciseTitle.value = ex.name
    exerciseVisible.value = true
  } catch (e) {
    console.error('Failed to resolve exercise:', e)
  }
}

function handleToggleAudio(path: string) {
  if (currentBook.value) {
    toggleAudio(currentBook.value.product_code, path)
  }
}

function closeReader() {
  appStore.currentBook = null
}

onMounted(() => {
  loadMetadata()
  window.addEventListener('keydown', handleKeyDown)
  if (readerRef.value) resizeObserver.observe(readerRef.value)
  readerStore.showUi()
})

onUnmounted(() => {
  audioCleanup()
  window.removeEventListener('keydown', handleKeyDown)
  resizeObserver.disconnect()
  readerStore.hideUi()
})
</script>

<template>
  <div ref="readerRef" class="reader-view flex h-full flex-col bg-gray-100 dark:bg-[#1f1f1f]">
    <div class="relative flex flex-1 overflow-hidden">
      <Transition name="slide-left">
        <ReaderTOC v-show="isUiVisible && !isSidebarCollapsed" :metadata="metadata" />
      </Transition>

      <ReaderCanvas
        :metadata="metadata"
        :loading="loading"
        :leftPageUrl="leftPageUrl"
        :rightPageUrl="rightPageUrl"
        :leftPageLabel="leftPageLabel"
        :rightPageLabel="rightPageLabel"
        :showHotspots="showHotspots"
        :canGoBack="canGoBack"
        :canGoForward="canGoForward"
        @overlayClick="handleOverlayClick"
        @goBack="goBack"
        @goForward="goForward"
      />
    </div>

    <Transition name="slide-down">
      <ReaderFooter
        v-show="isUiVisible"
        :displayIndex="displayIndex"
        :sortedPageLabels="sortedPageLabels"
        :currentPageAudioFiles="currentPageAudioFiles"
        :currentPageExercises="currentPageExercises"
        :isNarrow="isNarrow"
        @toggleAudio="handleToggleAudio"
        @openExercise="openExercise"
        @goBack="goBack"
        @goForward="goForward"
      />
    </Transition>

    <ReaderAudioPlayer />

    <ReaderExerciseModal />
    <ReaderDebugModal :metadata="metadata" :sortedPageLabels="sortedPageLabels" />
  </div>
</template>

<style>
/* Global styles for reader search and trees if needed */
.modern-executive-search .ant-input {
  border-radius: 24px !important;
  background-color: rgba(0, 0, 0, 0.025) !important;
  border: 1px solid rgba(0, 0, 0, 0.04) !important;
  font-size: 14px !important;
  padding: 8px 16px !important;
}
.dark .modern-executive-search .ant-input {
  background-color: rgba(255, 255, 255, 0.025) !important;
  border: 1px solid rgba(255, 255, 255, 0.06) !important;
}
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.05);
  border-radius: 10px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.05);
}

.slide-right-enter-active,
.slide-right-leave-active {
  transition: all 0.3s ease;
}
.slide-right-enter-from,
.slide-right-leave-to {
  transform: translateX(100%);
  opacity: 0;
}

.slide-left-enter-active,
.slide-left-leave-active {
  transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}
.slide-left-enter-from,
.slide-left-leave-to {
  transform: translateX(-100%);
  opacity: 0;
}

.slide-down-enter-active,
.slide-down-leave-active {
  transition: all 0.3s ease;
}
.slide-down-enter-from,
.slide-down-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>
