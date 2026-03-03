import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import ReaderExerciseModal from '../ReaderExerciseModal.vue'
import { useReaderStore } from '../../../stores/reader'

vi.mock('ant-design-vue', () => ({
  Modal: {
    template: '<div><slot /></div>',
    props: ['open'],
  },
}))

const modalStub = {
  template: '<div v-if="open" class="modal-stub" :data-width="width" :style="style"><slot /></div>',
  props: [
    'open',
    'style',
    'width',
    'title',
    'closable',
    'bodyStyle',
    'footer',
    'maskClosable',
    'destroyOnClose',
  ],
}

function setViewport(width: number, height: number) {
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    writable: true,
    value: width,
  })
  Object.defineProperty(window, 'innerHeight', {
    configurable: true,
    writable: true,
    value: height,
  })
  window.dispatchEvent(new Event('resize'))
}

function getAvailableSpace(width: number, height: number) {
  const marginRatio = width < 768 ? 0.035 : width < 1100 ? 0.04 : 0.05
  const margin = Math.round(Math.min(width, height) * marginRatio)
  return {
    availableWidth: Math.max(0, width - margin * 2),
    availableHeight: Math.max(0, height - margin * 2),
  }
}

function createPointerEvent(
  type: string,
  options: {
    clientX?: number
    clientY?: number
    pointerId?: number
    pointerType?: string
    button?: number
  } = {},
) {
  const event = new MouseEvent(type, {
    bubbles: true,
    cancelable: true,
    clientX: options.clientX ?? 0,
    clientY: options.clientY ?? 0,
    button: options.button ?? 0,
  }) as PointerEvent

  Object.defineProperties(event, {
    pointerId: {
      configurable: true,
      value: options.pointerId ?? 1,
    },
    pointerType: {
      configurable: true,
      value: options.pointerType ?? 'mouse',
    },
  })
  return event
}

function parseStylePx(style: string | undefined, property: string) {
  const match = style?.match(new RegExp(`${property}:\\s*([\\d.]+)px`))
  if (!match) {
    throw new Error(`Failed to parse "${property}" from style: ${style}`)
  }
  return Number(match[1])
}

function getModalMetrics(wrapper: VueWrapper) {
  const modal = wrapper.find('.modal-stub')
  const modalStyle = modal.attributes('style')
  const width = Number(modal.attributes('data-width'))

  const modalBody = wrapper
    .findAll('div')
    .find((node) => /height:\s*[\d.]+px/.test(node.attributes('style') || ''))
  if (!modalBody) {
    throw new Error('Modal body not found')
  }

  const bodyHeight = parseStylePx(modalBody.attributes('style'), 'height')
  return {
    width,
    height: bodyHeight + 40,
    top: parseStylePx(modalStyle, 'top'),
    left: parseStylePx(modalStyle, 'left'),
  }
}

function mountExerciseModal() {
  const readerStore = useReaderStore()
  readerStore.exerciseVisible = true
  readerStore.currentExerciseHtml = '<html><body>Test Exercise</body></html>'
  readerStore.currentExerciseUrl = 'eiuasset://localhost/exercise.html'
  readerStore.currentExerciseTitle = 'Test Title'

  const wrapper = mount(ReaderExerciseModal, {
    global: {
      stubs: {
        'a-modal': modalStub,
      },
    },
  })

  return { wrapper, readerStore }
}

describe('ReaderExerciseModal', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setViewport(1280, 800)
    vi.stubGlobal(
      'URL',
      Object.assign(URL, {
        createObjectURL: vi.fn(() => 'blob:test-exercise'),
        revokeObjectURL: vi.fn(),
      }),
    )
  })

  it('should render iframe when currentExerciseUrl is set', async () => {
    const { wrapper } = mountExerciseModal()

    const iframe = wrapper.find('iframe')
    expect(iframe.exists()).toBe(true)
    expect(iframe.attributes('src')).toBe('eiuasset://localhost/exercise.html')

    wrapper.unmount()
  })

  it('should respond to hello message from iframe', async () => {
    const { wrapper } = mountExerciseModal()

    const mockIframeSource = {
      postMessage: vi.fn(),
    }

    window.dispatchEvent(
      new MessageEvent('message', {
        data: { type: 'hello', id: 'test-id' },
        source: mockIframeSource as unknown as WindowProxy,
      }),
    )

    expect(mockIframeSource.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'hello-ack',
        id: 'test-id',
      }),
      '*',
    )

    wrapper.unmount()
  })

  it('closes modal on Escape key', async () => {
    const { readerStore, wrapper } = mountExerciseModal()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(readerStore.exerciseVisible).toBe(false)

    wrapper.unmount()
  })

  it('toggles maximize button text', async () => {
    const { wrapper } = mountExerciseModal()

    const maximizeButton = wrapper.find('[data-testid="exercise-maximize"]')
    expect(maximizeButton.exists()).toBe(true)
    expect(maximizeButton.attributes('aria-label')).toBe('Maximize')

    await maximizeButton.trigger('click')
    expect(maximizeButton.attributes('aria-label')).toBe('Restore')

    wrapper.unmount()
  })

  it('keeps phone initialization strategy while preserving 1:1.5 ratio', async () => {
    const viewport = { width: 390, height: 844 }
    setViewport(viewport.width, viewport.height)
    const { wrapper } = mountExerciseModal()
    await nextTick()

    const metrics = getModalMetrics(wrapper)
    const { availableWidth, availableHeight } = getAvailableSpace(viewport.width, viewport.height)
    expect(metrics.height / metrics.width).toBeCloseTo(1.5, 2)
    expect(metrics.width).toBeGreaterThan(availableWidth / 3)
    expect(metrics.height).toBeGreaterThan(availableHeight / 3)

    wrapper.unmount()
  })

  it('caps tablet initialization size to one third of current window', async () => {
    const viewport = { width: 900, height: 1200 }
    setViewport(viewport.width, viewport.height)
    const { wrapper } = mountExerciseModal()
    await nextTick()

    const metrics = getModalMetrics(wrapper)
    const { availableWidth, availableHeight } = getAvailableSpace(viewport.width, viewport.height)
    expect(metrics.height / metrics.width).toBeCloseTo(1.5, 2)
    expect(metrics.width).toBeLessThanOrEqual(availableWidth / 3 + 0.5)
    expect(metrics.height).toBeLessThanOrEqual(availableHeight + 0.5)

    wrapper.unmount()
  })

  it('caps desktop initialization size to one third of current window', async () => {
    const viewport = { width: 1440, height: 900 }
    setViewport(viewport.width, viewport.height)
    const { wrapper } = mountExerciseModal()
    await nextTick()

    const metrics = getModalMetrics(wrapper)
    const { availableWidth, availableHeight } = getAvailableSpace(viewport.width, viewport.height)
    expect(metrics.height / metrics.width).toBeCloseTo(1.5, 2)
    expect(metrics.width).toBeLessThanOrEqual(availableWidth / 3 + 0.5)
    expect(metrics.height).toBeLessThanOrEqual(availableHeight + 0.5)

    wrapper.unmount()
  })

  it('resets to initial size each time the modal is opened', async () => {
    setViewport(1440, 900)
    const { readerStore, wrapper } = mountExerciseModal()
    await nextTick()

    const initial = getModalMetrics(wrapper)
    const resizeHandle = wrapper.find('[data-testid="exercise-resize"]')
    expect(resizeHandle.exists()).toBe(true)

    resizeHandle.element.dispatchEvent(
      createPointerEvent('pointerdown', {
        clientX: 600,
        clientY: 500,
        pointerId: 11,
        pointerType: 'mouse',
        button: 0,
      }),
    )
    window.dispatchEvent(
      createPointerEvent('pointermove', {
        clientX: 760,
        clientY: 640,
        pointerId: 11,
        pointerType: 'mouse',
      }),
    )
    window.dispatchEvent(createPointerEvent('pointerup', { pointerId: 11, pointerType: 'mouse' }))
    await nextTick()

    const resized = getModalMetrics(wrapper)
    expect(resized.width).toBeGreaterThan(initial.width)
    expect(resized.height).toBeGreaterThan(initial.height)

    readerStore.exerciseVisible = false
    await nextTick()
    readerStore.exerciseVisible = true
    await nextTick()

    const reopened = getModalMetrics(wrapper)
    expect(Math.abs(reopened.width - initial.width)).toBeLessThan(0.5)
    expect(Math.abs(reopened.height - initial.height)).toBeLessThan(0.5)

    wrapper.unmount()
  })

  it('supports drag move via touch pointer events (iOS behavior)', async () => {
    setViewport(1280, 800)
    const { wrapper } = mountExerciseModal()
    await nextTick()

    const start = getModalMetrics(wrapper)
    const dragHeader = wrapper.find('.cursor-move')
    expect(dragHeader.exists()).toBe(true)

    dragHeader.element.dispatchEvent(
      createPointerEvent('pointerdown', {
        clientX: 220,
        clientY: 120,
        pointerId: 21,
        pointerType: 'touch',
      }),
    )
    window.dispatchEvent(
      createPointerEvent('pointermove', {
        clientX: 320,
        clientY: 220,
        pointerId: 21,
        pointerType: 'touch',
      }),
    )
    window.dispatchEvent(createPointerEvent('pointerup', { pointerId: 21, pointerType: 'touch' }))
    await nextTick()

    const moved = getModalMetrics(wrapper)
    expect(moved.left).not.toBe(start.left)
    expect(moved.top).not.toBe(start.top)

    wrapper.unmount()
  })
})
