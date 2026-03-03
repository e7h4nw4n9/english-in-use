import { onMounted, onUnmounted } from 'vue'

interface UseReaderShortcutsOptions {
  goBack: () => void
  goForward: () => void
  togglePlayback: () => void
  closeReader: () => void
  zoomIn: () => void
  zoomOut: () => void
  resetZoom: () => void
}

export function useReaderShortcuts({
  goBack,
  goForward,
  togglePlayback,
  closeReader,
  zoomIn,
  zoomOut,
  resetZoom,
}: UseReaderShortcutsOptions) {
  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'ArrowLeft') goBack()
    if (event.key === 'ArrowRight') goForward()
    if (event.key === ' ') {
      event.preventDefault()
      togglePlayback()
    }
    if (event.key === 'Escape') closeReader()

    if ((event.ctrlKey || event.metaKey) && (event.key === '=' || event.key === '+')) {
      event.preventDefault()
      zoomIn()
    }
    if ((event.ctrlKey || event.metaKey) && event.key === '-') {
      event.preventDefault()
      zoomOut()
    }
    if ((event.ctrlKey || event.metaKey) && event.key === '0') {
      event.preventDefault()
      resetZoom()
    }
  }

  onMounted(() => {
    window.addEventListener('keydown', handleKeyDown)
  })

  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeyDown)
  })

  return {
    handleKeyDown,
  }
}
