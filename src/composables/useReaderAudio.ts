import { storeToRefs } from 'pinia'
import { useReaderStore } from '../stores/reader'
import { resolveBookAsset } from '../lib/api/books'

const audioPlayer = new Audio()

export function useReaderAudio() {
  const readerStore = useReaderStore()
  const {
    currentAudioPath,
    isPlaying,
    playbackRate,
    audioCurrentTime,
    audioDuration,
    isAudioBarCollapsed,
  } = storeToRefs(readerStore)

  // Sync state from Audio object
  const onPlay = () => {
    isPlaying.value = true
  }
  const onPause = () => {
    isPlaying.value = false
  }
  const onEnded = () => {
    isPlaying.value = false
  }
  const onTimeUpdate = () => {
    audioCurrentTime.value = audioPlayer.currentTime
  }
  const onLoadedMetadata = () => {
    audioDuration.value = audioPlayer.duration
  }

  audioPlayer.addEventListener('play', onPlay)
  audioPlayer.addEventListener('pause', onPause)
  audioPlayer.addEventListener('ended', onEnded)
  audioPlayer.addEventListener('timeupdate', onTimeUpdate)
  audioPlayer.addEventListener('loadedmetadata', onLoadedMetadata)

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
  }

  async function toggleAudio(productCode: string, relPath: string) {
    if (currentAudioPath.value === relPath) {
      if (isPlaying.value) {
        audioPlayer.pause()
      } else {
        audioPlayer.play()
      }
    } else {
      try {
        const url = await resolveBookAsset(productCode, relPath)
        audioPlayer.src = url
        currentAudioPath.value = relPath
        audioPlayer.playbackRate = playbackRate.value
        audioPlayer.play()
      } catch (e) {
        console.error('Failed to play audio:', e)
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

  function togglePlay() {
    if (audioPlayer.paused) {
      audioPlayer.play()
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
    isAudioBarCollapsed,
    toggleAudio,
    togglePlay,
    seekAudio,
    handleAudioSliderChange,
    changePlaybackRate,
    formatTime,
    pauseAudio,
    cleanup,
    audioPlayer,
  }
}
