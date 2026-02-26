import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ReaderCanvas from '../ReaderCanvas.vue'
import { useReaderStore } from '../../../stores/reader'

describe('ReaderCanvas', () => {
  const mockMetadata = {
    product_code: 'test',
    pageWidth: 1000,
    pageHeight: 1400,
    pages: {
      '1': { overlays: [] },
      '2': { overlays: [] },
    },
  } as any

  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders correctly with zoom level', () => {
    const store = useReaderStore()
    store.setZoomLevel(1.5)
    const wrapper = mount(ReaderCanvas, {
      props: {
        metadata: mockMetadata,
        loading: false,
        leftPageUrl: 'left.jpg',
        rightPageUrl: 'right.jpg',
        leftPageLabel: '1',
        rightPageLabel: '2',
        showHotspots: true,
        canGoBack: true,
        canGoForward: true,
      },
      global: {
        stubs: ['a-spin', 'CustomerServiceOutlined', 'LinkOutlined'],
      },
    })

    const content = wrapper.find('.reader-content-container')
    expect(content.attributes('style')).toContain('transform: scale(1.5)')
  })

  it('handles mouse wheel zoom with Ctrl key', async () => {
    const store = useReaderStore()
    const wrapper = mount(ReaderCanvas, {
      props: {
        metadata: mockMetadata,
        loading: false,
        leftPageUrl: 'left.jpg',
        rightPageUrl: 'right.jpg',
        leftPageLabel: '1',
        rightPageLabel: '2',
        showHotspots: true,
        canGoBack: true,
        canGoForward: true,
      },
      global: {
        stubs: ['a-spin', 'CustomerServiceOutlined', 'LinkOutlined'],
      },
    })

    const scrollable = wrapper.find('.overflow-auto')

    // Zoom in
    const zoomInEvent = new WheelEvent('wheel', {
      ctrlKey: true,
      deltaY: -100,
      bubbles: true,
      cancelable: true,
    })
    scrollable.element.dispatchEvent(zoomInEvent)
    expect(store.zoomLevel).toBeGreaterThan(1.0)

    // Zoom out
    const currentZoom = store.zoomLevel
    const zoomOutEvent = new WheelEvent('wheel', {
      ctrlKey: true,
      deltaY: 100,
      bubbles: true,
      cancelable: true,
    })
    scrollable.element.dispatchEvent(zoomOutEvent)
    expect(store.zoomLevel).toBeLessThan(currentZoom)
  })
})
