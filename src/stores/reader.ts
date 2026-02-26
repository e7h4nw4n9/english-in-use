import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export const useReaderStore = defineStore('reader', () => {
  const isSidebarCollapsed = ref(true)
  const viewMode = ref<'single' | 'spread'>('single')
  const zoomLevel = ref(1.0)
  const currentPageLabel = ref('12') // Default start page based on our data observation
  const spreadOffset = ref(0)

  // Audio State
  const currentAudioPath = ref<string | null>(null)
  const isPlaying = ref(false)
  const playbackRate = ref(1.0)
  const audioCurrentTime = ref(0)
  const audioDuration = ref(0)
  const isAudioBarCollapsed = ref(false)
  const showHotspots = ref(true)

  // Exercise State
  const exerciseVisible = ref(false)
  const currentExerciseUrl = ref('')
  const currentExerciseTitle = ref('')
  const currentUnitName = ref('')
  const debugVisible = ref(false)
  const resourceDrawerVisible = ref(false)
  const isUiVisible = ref(true)

  const showUi = () => {
    isUiVisible.value = true
  }

  const hideUi = () => {
    isUiVisible.value = false
  }

  const setSidebarCollapsed = (collapsed: boolean) => {
    isSidebarCollapsed.value = collapsed
  }

  const setViewMode = (mode: 'single' | 'spread') => {
    viewMode.value = mode
  }

  const setZoomLevel = (level: number) => {
    zoomLevel.value = Math.max(0.25, Math.min(5.0, level))
  }

  const zoomIn = (step = 0.05) => {
    setZoomLevel(zoomLevel.value + step)
  }

  const zoomOut = (step = 0.05) => {
    setZoomLevel(zoomLevel.value - step)
  }

  const resetZoom = () => {
    zoomLevel.value = 1.0
  }

  const setCurrentPageLabel = (label: string) => {
    currentPageLabel.value = label
  }

  const resetAudio = () => {
    currentAudioPath.value = null
    isPlaying.value = false
    audioCurrentTime.value = 0
    audioDuration.value = 0
  }

  const toggleHotspots = () => {
    showHotspots.value = !showHotspots.value
  }

  const jumpToIndex = (index: number, labels: string[]) => {
    if (index >= 0 && index < labels.length) {
      currentPageLabel.value = labels[index]
    }
  }

  return {
    isSidebarCollapsed,
    viewMode,
    zoomLevel,
    currentPageLabel,
    spreadOffset,
    currentAudioPath,
    isPlaying,
    playbackRate,
    audioCurrentTime,
    audioDuration,
    isAudioBarCollapsed,
    showHotspots,
    exerciseVisible,
    currentExerciseUrl,
    currentExerciseTitle,
    currentUnitName,
    debugVisible,
    resourceDrawerVisible,
    isUiVisible,
    showUi,
    hideUi,
    setSidebarCollapsed,
    setViewMode,
    setZoomLevel,
    zoomIn,
    zoomOut,
    resetZoom,
    setCurrentPageLabel,
    resetAudio,
    toggleHotspots,
    jumpToIndex,
  }
})
