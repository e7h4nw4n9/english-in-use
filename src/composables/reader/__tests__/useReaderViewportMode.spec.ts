import { describe, expect, it, vi, beforeEach } from 'vitest'
import { ref } from 'vue'
import { useReaderViewportMode } from '../useReaderViewportMode'

let lastObserver: {
  callback: (entries: Array<{ contentRect: { width: number; height: number } }>) => void
  observe: ReturnType<typeof vi.fn>
  disconnect: ReturnType<typeof vi.fn>
} | null = null

class MockResizeObserver {
  observe = vi.fn()
  disconnect = vi.fn()
  callback: (entries: Array<{ contentRect: { width: number; height: number } }>) => void

  constructor(
    callback: (entries: Array<{ contentRect: { width: number; height: number } }>) => void,
  ) {
    this.callback = callback
    lastObserver = {
      callback,
      observe: this.observe,
      disconnect: this.disconnect,
    }
  }
}

vi.stubGlobal('ResizeObserver', MockResizeObserver as unknown as typeof ResizeObserver)

describe('useReaderViewportMode', () => {
  beforeEach(() => {
    lastObserver = null
  })

  it('forces spread mode to single on narrow width', () => {
    const viewMode = ref<'single' | 'spread'>('spread')
    const zoomLevel = ref(1.6)
    const { isNarrow } = useReaderViewportMode({ viewMode, zoomLevel, narrowThreshold: 800 })

    lastObserver?.callback([{ contentRect: { width: 600, height: 900 } }])

    expect(isNarrow.value).toBe(true)
    expect(viewMode.value).toBe('single')
    expect(zoomLevel.value).toBe(1)
  })

  it('forces iPad portrait mode to single at 768px width', () => {
    const viewMode = ref<'single' | 'spread'>('spread')
    const zoomLevel = ref(1.3)
    const { isNarrow } = useReaderViewportMode({
      viewMode,
      zoomLevel,
      narrowThreshold: 768,
      isIpadOverride: true,
    })

    lastObserver?.callback([{ contentRect: { width: 768, height: 1024 } }])

    expect(isNarrow.value).toBe(true)
    expect(viewMode.value).toBe('single')
    expect(zoomLevel.value).toBe(1)
  })

  it('handles observe and disconnect lifecycle', () => {
    const viewMode = ref<'single' | 'spread'>('single')
    const { observe, disconnect, isNarrow } = useReaderViewportMode({ viewMode })

    observe(null)
    expect(lastObserver?.observe).not.toHaveBeenCalled()

    const element = document.createElement('div')
    observe(element)
    expect(lastObserver?.observe).toHaveBeenCalledWith(element)

    lastObserver?.callback([{ contentRect: { width: 1000, height: 700 } }])
    expect(isNarrow.value).toBe(false)

    disconnect()
    expect(lastObserver?.disconnect).toHaveBeenCalledTimes(1)
  })
})
