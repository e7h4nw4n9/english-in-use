import { storeToRefs } from 'pinia'
import { useReaderStore } from '../stores/reader'
import { resolveBookAsset } from '../lib/api/books'
import { getReadableCommandError } from '../lib/error'

interface ReaderAudioRuntime {
  audioPlayer: HTMLAudioElement
  playRequestVersion: number
  listenersAttached: boolean
  stopLock: boolean
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
    listenersAttached: false,
    stopLock: false,
  })

const audioPlayer = runtime.audioPlayer

function getAudioErrorMessage() {
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

function attachAudioListeners() {
  if (runtime.listenersAttached) return
  runtime.listenersAttached = true

  audioPlayer.addEventListener('play', () => {
    if (runtime.stopLock) {
      audioPlayer.pause()
      return
    }
    useReaderStore().isPlaying = true
  })
  audioPlayer.addEventListener('pause', () => {
    if (runtime.stopLock) return
    const store = useReaderStore()
    store.isPlaying = false
    store.isAudioLoading = false
  })
  audioPlayer.addEventListener('ended', () => {
    if (runtime.stopLock) return
    const store = useReaderStore()
    store.isPlaying = false
    store.isAudioLoading = false
  })
  audioPlayer.addEventListener('timeupdate', () => {
    if (runtime.stopLock) return
    useReaderStore().audioCurrentTime = audioPlayer.currentTime
  })
  audioPlayer.addEventListener('loadedmetadata', () => {
    if (runtime.stopLock) return
    useReaderStore().audioDuration = Number.isFinite(audioPlayer.duration)
      ? audioPlayer.duration
      : 0
  })
  audioPlayer.addEventListener('loadstart', () => {
    if (runtime.stopLock) return
    const store = useReaderStore()
    store.audioError = null
    store.isAudioLoading = true
  })
  for (const eventName of ['waiting', 'stalled']) {
    audioPlayer.addEventListener(eventName, () => {
      if (runtime.stopLock) return
      useReaderStore().isAudioLoading = true
    })
  }
  audioPlayer.addEventListener('canplay', () => {
    if (runtime.stopLock) return
    useReaderStore().isAudioLoading = false
  })
  audioPlayer.addEventListener('playing', () => {
    if (runtime.stopLock) return
    const store = useReaderStore()
    store.isAudioLoading = false
    store.audioError = null
  })
  audioPlayer.addEventListener('error', () => {
    if (runtime.stopLock) return
    const store = useReaderStore()
    store.isAudioLoading = false
    store.audioError = getAudioErrorMessage()
  })
}

/** 管理阅读器音频加载、播放、进度和播放速率。 */
export function useReaderAudio() {
  attachAudioListeners()
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

  /** 播放音频，并忽略已被后续操作淘汰的异步错误。 */
  function playSafely(requestVersion = runtime.playRequestVersion) {
    const playPromise = audioPlayer.play()
    if (!playPromise) return

    void playPromise.catch((error) => {
      if (requestVersion !== runtime.playRequestVersion || runtime.stopLock) return
      console.error('Failed to play audio:', error)
      isAudioLoading.value = false
      audioError.value = getReadableCommandError(error)
    })
  }

  function prepareForExplicitPlay() {
    runtime.stopLock = false
    audioPlayer.muted = false
    if (audioPlayer.volume === 0) {
      audioPlayer.volume = 1
    }
  }

  /** 切换指定音频，使用请求版本阻止旧解析结果覆盖当前状态。 */
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
      const requestVersion = ++runtime.playRequestVersion
      try {
        audioError.value = null
        isAudioLoading.value = true
        const url = await resolveBookAsset(productCode, relPath)
        if (requestVersion !== runtime.playRequestVersion) {
          return
        }
        audioPlayer.src = url
        currentAudioPath.value = relPath
        audioPlayer.playbackRate = playbackRate.value
        prepareForExplicitPlay()
        playSafely(requestVersion)
      } catch (e) {
        if (requestVersion !== runtime.playRequestVersion) return
        console.error('Failed to play audio:', e)
        isAudioLoading.value = false
        audioError.value = getReadableCommandError(e)
      }
    }
  }

  function seekAudio(seconds: number) {
    const duration = Number.isFinite(audioPlayer.duration) ? audioPlayer.duration : 0
    const currentTime = Number.isFinite(audioPlayer.currentTime) ? audioPlayer.currentTime : 0
    audioPlayer.currentTime = Math.max(0, Math.min(duration, currentTime + seconds))
  }

  function handleAudioSliderChange(val: number) {
    if (Number.isFinite(val)) {
      audioPlayer.currentTime = val
    }
  }

  function changePlaybackRate(rate: number) {
    playbackRate.value = rate
    audioPlayer.playbackRate = rate
  }

  function formatTime(seconds: number) {
    if (!Number.isFinite(seconds) || seconds < 0) return '0:00'
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  function pauseAudio() {
    audioPlayer.pause()
  }

  function stopAndResetAudio() {
    runtime.stopLock = true
    runtime.playRequestVersion++
    audioPlayer.muted = true
    audioPlayer.volume = 0
    audioPlayer.pause()
    try {
      audioPlayer.currentTime = 0
    } catch {
      // 部分媒体引擎在切换资源期间会抛出异常，此处只需继续清理状态。
    }
    audioPlayer.removeAttribute('src')
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
    cleanup: stopAndResetAudio,
    audioPlayer,
  }
}
