import { storeToRefs } from 'pinia'
import { useReaderStore } from '../stores/reader'
import { resolveBookAsset } from '../lib/api/books'
import { getReadableCommandError } from '../lib/error'

interface ReaderAudioRuntime {
  audioPlayer: HTMLAudioElement
  playRequestVersion: number
}

type ReaderAudioGlobal = typeof globalThis & {
  __readerAudioRuntime?: ReaderAudioRuntime
}

const readerAudioGlobal = globalThis as ReaderAudioGlobal
const runtime =
  readerAudioGlobal.__readerAudioRuntime ??
  (readerAudioGlobal.__readerAudioRuntime = {
    audioPlayer: new Audio(),
    playRequestVersion: 0,
  })

const audioPlayer = runtime.audioPlayer
let stopLock = false

export function useReaderAudio() {
  const readerStore = useReaderStore()
  const {
    currentAudioPath,
    isPlaying,
    playbackRate,
    audioCurrentTime,
    audioDuration,
    isAudioLoading,
    audioError,
    isAudioBarCollapsed,
  } = storeToRefs(readerStore)

  const getAudioErrorMessage = () => {
    const mediaError = audioPlayer.error
    if (!mediaError) return 'Audio failed to load'
    switch (mediaError.code) {
      case 1:
        return 'Audio loading was aborted'
      case 2:
        return 'Network error while loading audio'
      case 3:
        return 'Audio decode error'
      case 4:
        return 'Audio source is not supported'
      default:
        return 'Audio failed to load'
    }
  }

  // Sync state from Audio object
  const onPlay = () => {
    if (stopLock) {
      audioPlayer.pause()
      return
    }

    isPlaying.value = true
  }
  const onPause = () => {
    isPlaying.value = false
    isAudioLoading.value = false
  }
  const onEnded = () => {
    isPlaying.value = false
    isAudioLoading.value = false
  }
  const onTimeUpdate = () => {
    audioCurrentTime.value = audioPlayer.currentTime
  }
  const onLoadedMetadata = () => {
    audioDuration.value = audioPlayer.duration
  }
  const onLoadStart = () => {
    audioError.value = null
    isAudioLoading.value = true
  }
  const onWaiting = () => {
    isAudioLoading.value = true
  }
  const onCanPlay = () => {
    isAudioLoading.value = false
  }
  const onPlaying = () => {
    isAudioLoading.value = false
    audioError.value = null
  }
  const onStalled = () => {
    isAudioLoading.value = true
  }
  const onError = () => {
    isAudioLoading.value = false
    audioError.value = getAudioErrorMessage()
  }

  audioPlayer.addEventListener('play', onPlay)
  audioPlayer.addEventListener('pause', onPause)
  audioPlayer.addEventListener('ended', onEnded)
  audioPlayer.addEventListener('timeupdate', onTimeUpdate)
  audioPlayer.addEventListener('loadedmetadata', onLoadedMetadata)
  audioPlayer.addEventListener('loadstart', onLoadStart)
  audioPlayer.addEventListener('waiting', onWaiting)
  audioPlayer.addEventListener('canplay', onCanPlay)
  audioPlayer.addEventListener('playing', onPlaying)
  audioPlayer.addEventListener('stalled', onStalled)
  audioPlayer.addEventListener('error', onError)

  // Note: We don't remove listeners onUnmounted here if multiple components use this.
  // Instead, we might want a way to manage lifecycle or just keep it active as a singleton service.
  // But for safety within Vue components, we can expose a cleanup.
  const cleanup = () => {
    audioPlayer.pause()
    audioPlayer.removeEventListener('play', onPlay)
    audioPlayer.removeEventListener('pause', onPause)
    audioPlayer.removeEventListener('ended', onEnded)
    audioPlayer.removeEventListener('timeupdate', onTimeUpdate)
    audioPlayer.removeEventListener('loadedmetadata', onLoadedMetadata)
    audioPlayer.removeEventListener('loadstart', onLoadStart)
    audioPlayer.removeEventListener('waiting', onWaiting)
    audioPlayer.removeEventListener('canplay', onCanPlay)
    audioPlayer.removeEventListener('playing', onPlaying)
    audioPlayer.removeEventListener('stalled', onStalled)
    audioPlayer.removeEventListener('error', onError)
  }

  function playSafely() {
    const playPromise = audioPlayer.play()
    if (!playPromise) return

    void playPromise.catch((error) => {
      console.error('Failed to play audio:', error)
      isAudioLoading.value = false
      audioError.value = getReadableCommandError(error)
    })
  }

  function prepareForExplicitPlay() {
    stopLock = false
    audioPlayer.muted = false
    if (audioPlayer.volume === 0) {
      audioPlayer.volume = 1
    }
  }

  async function toggleAudio(productCode: string, relPath: string) {
    if (currentAudioPath.value === relPath) {
      if (isPlaying.value) {
        audioPlayer.pause()
      } else {
        audioError.value = null
        isAudioLoading.value = audioPlayer.readyState < 3
        prepareForExplicitPlay()
        playSafely()
      }
    } else {
      try {
        const requestVersion = ++runtime.playRequestVersion
        audioError.value = null
        isAudioLoading.value = true
        const url = await resolveBookAsset(productCode, relPath)
        if (requestVersion !== runtime.playRequestVersion) {
          isAudioLoading.value = false
          return
        }
        audioPlayer.src = url
        currentAudioPath.value = relPath
        audioPlayer.playbackRate = playbackRate.value
        prepareForExplicitPlay()
        playSafely()
      } catch (e) {
        console.error('Failed to play audio:', e)
        isAudioLoading.value = false
        audioError.value = getReadableCommandError(e)
      }
    }
  }

  function seekAudio(seconds: number) {
    audioPlayer.currentTime = Math.max(
      0,
      Math.min(audioPlayer.duration, audioPlayer.currentTime + seconds),
    )
  }

  function handleAudioSliderChange(val: number) {
    audioPlayer.currentTime = val
  }

  function changePlaybackRate(rate: number) {
    playbackRate.value = rate
    audioPlayer.playbackRate = rate
  }

  function formatTime(seconds: number) {
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  function pauseAudio() {
    audioPlayer.pause()
  }

  function stopAndResetAudio() {
    stopLock = true
    runtime.playRequestVersion++
    audioPlayer.muted = true
    audioPlayer.volume = 0
    audioPlayer.pause()
    try {
      audioPlayer.currentTime = 0
    } catch {
      // Some engines may throw while media is transitioning source.
    }
    audioPlayer.removeAttribute('src')
    audioPlayer.src = ''
    audioPlayer.load()
    readerStore.resetAudio()
    setTimeout(() => {
      audioPlayer.pause()
    }, 0)
  }

  function togglePlay() {
    if (audioPlayer.paused) {
      audioError.value = null
      isAudioLoading.value = audioPlayer.readyState < 3
      prepareForExplicitPlay()
      playSafely()
    } else {
      audioPlayer.pause()
    }
  }

  return {
    currentAudioPath,
    isPlaying,
    playbackRate,
    audioCurrentTime,
    audioDuration,
    isAudioLoading,
    audioError,
    isAudioBarCollapsed,
    toggleAudio,
    togglePlay,
    seekAudio,
    handleAudioSliderChange,
    changePlaybackRate,
    formatTime,
    pauseAudio,
    stopAndResetAudio,
    cleanup,
    audioPlayer,
  }
}
