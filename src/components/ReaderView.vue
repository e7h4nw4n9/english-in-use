<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '../stores/app'
import { useReaderStore } from '../stores/reader'
import { useReaderAudio } from '../composables/useReaderAudio'
import { useReaderMetadata } from '../composables/useReaderMetadata'
import { useReaderShortcuts } from '../composables/reader/useReaderShortcuts'
import { useReaderTocContext } from '../composables/reader/useReaderTocContext'
import { useReaderExerciseLoader } from '../composables/reader/useReaderExerciseLoader'
import { useReaderViewportMode } from '../composables/reader/useReaderViewportMode'
import { useReaderOverlayActions } from '../composables/reader/useReaderOverlayActions'
import { useI18n } from 'vue-i18n'

// Components
import ReaderTOC from './reader/ReaderTOC.vue'
import ReaderCanvas from './reader/ReaderCanvas.vue'
import ReaderFooter from './reader/ReaderFooter.vue'
import ReaderAudioPlayer from './reader/ReaderAudioPlayer.vue'
import ReaderExerciseModal from './reader/ReaderExerciseModal.vue'
import ReaderDebugModal from './reader/ReaderDebugModal.vue'

const appStore = useAppStore()
const readerStore = useReaderStore()
const { t } = useI18n()
const { currentBook } = storeToRefs(appStore)
const readerRef = ref<HTMLElement | null>(null)
const {
  currentPageLabel,
  viewMode,
  zoomLevel,
  exerciseVisible,
  currentExerciseUrl,
  currentExerciseHtml,
  currentExerciseTitle,
  currentExerciseResourceId,
  showHotspots,
  isUiVisible,
  isPlaying,
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

const { toggleAudio, stopAndResetAudio, cleanup: audioCleanup } = useReaderAudio()
const fallbackUnitTitle = computed(() => currentBook.value?.title || '')
const exerciseDebugPanelEnabled = true
const { currentUnitName, currentPageAudioFiles } = useReaderTocContext({
  metadata,
  currentPageLabel,
  leftPageLabel,
  rightPageLabel,
  viewMode,
  sortedPageLabels,
  fallbackUnitTitle,
})
const { openExercise } = useReaderExerciseLoader({
  currentBook,
  appStore,
  exerciseVisible,
  currentExerciseUrl,
  currentExerciseHtml,
  currentExerciseTitle,
  currentExerciseResourceId,
  t,
})
const exerciseDebugMeta = computed(() => ({
  productCode: currentBook.value?.product_code || '',
  pageLabel: currentPageLabel.value || '',
  unitName: currentUnitName.value || fallbackUnitTitle.value || '',
}))
const { handleOverlayClick } = useReaderOverlayActions({
  currentBook,
  currentPageLabel,
  openExercise,
  toggleAudio,
})

const { isNarrow, observe, disconnect } = useReaderViewportMode({
  viewMode,
  zoomLevel,
})

watch(
  currentUnitName,
  (unitName) => {
    readerStore.currentUnitName = unitName
  },
  { immediate: true },
)

watch(currentPageLabel, (newLabel, oldLabel) => {
  if (newLabel !== oldLabel) {
    stopAndResetAudio()
  }
})

function handleToggleAudio(path: string) {
  if (currentBook.value) {
    toggleAudio(currentBook.value.product_code, path)
  }
}

function closeReader() {
  appStore.currentBook = null
}

useReaderShortcuts({
  goBack,
  goForward,
  togglePlayback: () => {
    isPlaying.value = !isPlaying.value
  },
  closeReader,
  zoomIn: () => readerStore.zoomIn(),
  zoomOut: () => readerStore.zoomOut(),
  resetZoom: () => readerStore.resetZoom(),
})

onMounted(() => {
  loadMetadata()
  observe(readerRef.value)
  readerStore.showUi()
})

onUnmounted(() => {
  audioCleanup()
  disconnect()
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
        :isNarrow="isNarrow"
        @toggleAudio="handleToggleAudio"
        @openExercise="openExercise"
        @goBack="goBack"
        @goForward="goForward"
      />
    </Transition>

    <ReaderAudioPlayer />

    <ReaderExerciseModal
      :enableDebugPanel="exerciseDebugPanelEnabled"
      :debugMeta="exerciseDebugMeta"
    />
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
