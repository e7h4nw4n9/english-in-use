import { ref, type Ref } from 'vue'

interface UseReaderViewportModeOptions {
  viewMode: Ref<'single' | 'spread'>
  zoomLevel?: Ref<number>
  narrowThreshold?: number
  isIpadOverride?: boolean
}

function isIpadDevice() {
  if (typeof navigator === 'undefined') return false
  const ua = navigator.userAgent || ''
  const isLegacyIpad = /iPad/i.test(ua)
  const isIpadOs = navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1
  return isLegacyIpad || isIpadOs
}

export function useReaderViewportMode({
  viewMode,
  zoomLevel,
  narrowThreshold = 768,
  isIpadOverride,
}: UseReaderViewportModeOptions) {
  const isNarrow = ref(false)
  const isIpad = isIpadOverride ?? isIpadDevice()

  const resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const { width, height } = entry.contentRect
      const isIpadPortrait = isIpad && height >= width
      isNarrow.value = width < narrowThreshold || isIpadPortrait
      if (isIpadPortrait && zoomLevel && zoomLevel.value !== 1) {
        zoomLevel.value = 1
      }
      if (isNarrow.value && viewMode.value === 'spread') {
        viewMode.value = 'single'
        if (zoomLevel && zoomLevel.value !== 1) {
          zoomLevel.value = 1
        }
      }
    }
  })

  function observe(target: Element | null) {
    if (!target) return
    resizeObserver.observe(target)
  }

  function disconnect() {
    resizeObserver.disconnect()
  }

  return {
    isNarrow,
    observe,
    disconnect,
  }
}
