import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ReaderCanvas from '../ReaderCanvas.vue'
import { useReaderStore } from '../../../stores/reader'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

function createTouchEvent(
  type: string,
  touches: Array<{ clientX: number; clientY: number }>,
): TouchEvent {
  const event = new Event(type, { bubbles: true, cancelable: true }) as TouchEvent
  Object.defineProperty(event, 'touches', {
    value: touches as unknown as TouchList,
    configurable: true,
  })
  return event
}

const originalNavigatorState = {
  userAgent: navigator.userAgent,
  platform: navigator.platform,
  maxTouchPoints: navigator.maxTouchPoints,
}

function setNavigatorState(overrides: Partial<typeof originalNavigatorState>) {
  Object.defineProperty(window.navigator, 'userAgent', {
    configurable: true,
    value: overrides.userAgent ?? originalNavigatorState.userAgent,
  })
  Object.defineProperty(window.navigator, 'platform', {
    configurable: true,
    value: overrides.platform ?? originalNavigatorState.platform,
  })
  Object.defineProperty(window.navigator, 'maxTouchPoints', {
    configurable: true,
    value: overrides.maxTouchPoints ?? originalNavigatorState.maxTouchPoints,
  })
}

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
    setNavigatorState({})
  })

  afterEach(() => {
    vi.useRealTimers()
    setNavigatorState({})
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

  it('uses scroll container clearance instead of hardcoded content bottom padding', () => {
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

    const scrollContainer = wrapper.find('.reader-scroll-container')
    expect(scrollContainer.exists()).toBe(true)

    const content = wrapper.find('.reader-content-container')
    expect(content.classes()).not.toContain('pb-48')
  })

  it('uses full-width page surface in single mode', () => {
    const wrapper = mount(ReaderCanvas, {
      props: {
        metadata: mockMetadata,
        loading: false,
        leftPageUrl: 'left.jpg',
        rightPageUrl: '',
        leftPageLabel: '1',
        rightPageLabel: '',
        showHotspots: true,
        canGoBack: true,
        canGoForward: true,
      },
      global: {
        stubs: ['a-spin', 'CustomerServiceOutlined', 'LinkOutlined'],
      },
    })

    const pageSurface = wrapper.find('.page-surface')
    const style = pageSurface.attributes('style')
    expect(style).toContain('width: 100%')
    expect(style).toContain('max-width: 100%')
    expect(style).toContain('flex: 1 0 100%')
  })

  it('renders exercise overlay with KeyOutlined icon', () => {
    const metadataWithExercise = {
      ...mockMetadata,
      pages: {
        '1': {
          overlays: [
            {
              x: 100,
              y: 100,
              w: 50,
              h: 50,
              type: 'exercise',
              exercise: { name: 'Ex 1', resource_id: 'ex1' },
            },
          ],
        },
      },
    }
    const wrapper = mount(ReaderCanvas, {
      props: {
        metadata: metadataWithExercise,
        loading: false,
        leftPageUrl: 'left.jpg',
        rightPageUrl: '',
        leftPageLabel: '1',
        rightPageLabel: '',
        showHotspots: true,
        canGoBack: true,
        canGoForward: true,
      },
      global: {
        stubs: {
          'a-spin': true,
          CustomerServiceOutlined: true,
          LinkOutlined: true,
          KeyOutlined: { template: '<span class="key-icon-stub" />' },
        },
      },
    })

    expect(wrapper.find('.key-icon-stub').exists()).toBe(true)
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

  it('handles pinch zoom on touch devices', () => {
    vi.useFakeTimers()
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

    scrollable.element.dispatchEvent(
      createTouchEvent('touchstart', [
        { clientX: 0, clientY: 0 },
        { clientX: 100, clientY: 0 },
      ]),
    )
    scrollable.element.dispatchEvent(
      createTouchEvent('touchmove', [
        { clientX: 0, clientY: 0 },
        { clientX: 150, clientY: 0 },
      ]),
    )
    vi.advanceTimersByTime(16)
    expect(store.zoomLevel).toBeGreaterThan(1.0)
    expect(store.zoomLevel).toBeLessThan(1.5)

    const zoomAfterPinchIn = store.zoomLevel
    scrollable.element.dispatchEvent(createTouchEvent('touchend', []))

    scrollable.element.dispatchEvent(
      createTouchEvent('touchstart', [
        { clientX: 0, clientY: 0 },
        { clientX: 150, clientY: 0 },
      ]),
    )
    scrollable.element.dispatchEvent(
      createTouchEvent('touchmove', [
        { clientX: 0, clientY: 0 },
        { clientX: 90, clientY: 0 },
      ]),
    )
    vi.advanceTimersByTime(16)
    expect(store.zoomLevel).toBeLessThan(zoomAfterPinchIn)
  })

  it('ignores tiny pinch jitter to avoid zoom flicker', () => {
    vi.useFakeTimers()
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

    scrollable.element.dispatchEvent(
      createTouchEvent('touchstart', [
        { clientX: 0, clientY: 0 },
        { clientX: 100, clientY: 0 },
      ]),
    )

    scrollable.element.dispatchEvent(
      createTouchEvent('touchmove', [
        { clientX: 0, clientY: 0 },
        { clientX: 101, clientY: 0 },
      ]),
    )
    vi.advanceTimersByTime(16)
    expect(store.zoomLevel).toBe(1.0)

    scrollable.element.dispatchEvent(
      createTouchEvent('touchmove', [
        { clientX: 0, clientY: 0 },
        { clientX: 103, clientY: 0 },
      ]),
    )
    vi.advanceTimersByTime(16)
    expect(store.zoomLevel).toBeGreaterThan(1.0)
  })

  it('supports iOS swipe left to go forward', () => {
    setNavigatorState({
      userAgent:
        'Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
      platform: 'iPad',
      maxTouchPoints: 5,
    })

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
    scrollable.element.dispatchEvent(
      createTouchEvent('touchstart', [{ clientX: 220, clientY: 180 }]),
    )
    scrollable.element.dispatchEvent(
      createTouchEvent('touchmove', [{ clientX: 140, clientY: 184 }]),
    )
    scrollable.element.dispatchEvent(createTouchEvent('touchend', []))

    expect(wrapper.emitted('goForward')).toHaveLength(1)
  })

  it('supports iOS swipe right to go back', () => {
    setNavigatorState({
      userAgent:
        'Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
      platform: 'iPad',
      maxTouchPoints: 5,
    })

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
    scrollable.element.dispatchEvent(
      createTouchEvent('touchstart', [{ clientX: 120, clientY: 200 }]),
    )
    scrollable.element.dispatchEvent(
      createTouchEvent('touchmove', [{ clientX: 200, clientY: 196 }]),
    )
    scrollable.element.dispatchEvent(createTouchEvent('touchend', []))

    expect(wrapper.emitted('goBack')).toHaveLength(1)
  })

  it('does not trigger swipe page turn on non-iOS devices', () => {
    setNavigatorState({
      userAgent:
        'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36',
      platform: 'Linux x86_64',
      maxTouchPoints: 0,
    })

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
    scrollable.element.dispatchEvent(
      createTouchEvent('touchstart', [{ clientX: 220, clientY: 180 }]),
    )
    scrollable.element.dispatchEvent(
      createTouchEvent('touchmove', [{ clientX: 120, clientY: 180 }]),
    )
    scrollable.element.dispatchEvent(createTouchEvent('touchend', []))

    expect(wrapper.emitted('goForward')).toBeUndefined()
    expect(wrapper.emitted('goBack')).toBeUndefined()
  })
})
