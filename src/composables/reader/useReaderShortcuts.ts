import { onMounted, onUnmounted } from 'vue'

interface UseReaderShortcutsOptions {
  isBlocked?: () => boolean
  goBack: () => void
  goForward: () => void
  togglePlayback: () => void
  closeReader: () => void
  zoomIn: () => void
  zoomOut: () => void
  resetZoom: () => void
}

/**
 * 注册阅读器键盘翻页、播放、关闭和缩放快捷键。
 * @param options - 快捷键对应的动作回调。
 */
export function useReaderShortcuts({
  isBlocked,
  goBack,
  goForward,
  togglePlayback,
  closeReader,
  zoomIn,
  zoomOut,
  resetZoom,
}: UseReaderShortcutsOptions) {
  const handleKeyDown = (event: KeyboardEvent) => {
    const target = event.target
    if (
      Array.from(document.querySelectorAll('[role="dialog"]')).some(
        (dialog) => dialog.getClientRects().length > 0,
      )
    )
      return
    if (
      event.defaultPrevented ||
      isBlocked?.() ||
      (target instanceof Element &&
        target.closest('input, textarea, select, [contenteditable="true"], [role="dialog"]'))
    )
      return
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
