import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ReaderExerciseModal from '../ReaderExerciseModal.vue'
import { useReaderStore } from '../../../stores/reader'

// Mock Ant Design Vue components
vi.mock('ant-design-vue', () => ({
  Modal: {
    template: '<div><slot /></div>',
    props: ['open'],
  },
}))

describe('ReaderExerciseModal', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.stubGlobal(
      'URL',
      Object.assign(URL, {
        createObjectURL: vi.fn(() => 'blob:test-exercise'),
        revokeObjectURL: vi.fn(),
      }),
    )
  })

  it('should render iframe with blob src when currentExerciseHtml is set', async () => {
    const readerStore = useReaderStore()
    readerStore.exerciseVisible = true
    readerStore.currentExerciseHtml = '<html><body>Test Exercise</body></html>'
    readerStore.currentExerciseTitle = 'Test Title'

    const wrapper = mount(ReaderExerciseModal, {
      global: {
        stubs: {
          'a-modal': {
            template: '<div v-if="open"><slot /></div>',
            props: ['open'],
          },
        },
      },
    })

    const iframe = wrapper.find('iframe')
    expect(iframe.exists()).toBe(true)
    expect(iframe.attributes('src')).toBe('blob:test-exercise')
  })

  it('should respond to hello message from iframe', async () => {
    const readerStore = useReaderStore()
    readerStore.exerciseVisible = true
    readerStore.currentExerciseHtml = '<html><body>Test</body></html>'

    mount(ReaderExerciseModal, {
      global: {
        stubs: {
          'a-modal': {
            template: '<div v-if="open"><slot /></div>',
            props: ['open'],
          },
        },
      },
    })

    // Simulate message from iframe
    const mockIframeSource = {
      postMessage: vi.fn(),
    }

    window.dispatchEvent(
      new MessageEvent('message', {
        data: { type: 'hello', id: 'test-id' },
        source: mockIframeSource as unknown as WindowProxy,
      }),
    )

    // Check if we responded with hello-ack
    expect(mockIframeSource.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'hello-ack',
        id: 'test-id',
      }),
      '*',
    )
  })

  it('closes modal on Escape key', async () => {
    const readerStore = useReaderStore()
    readerStore.exerciseVisible = true
    readerStore.currentExerciseHtml = '<html><body>Test</body></html>'

    mount(ReaderExerciseModal, {
      global: {
        stubs: {
          'a-modal': {
            template: '<div v-if="open"><slot /></div>',
            props: ['open'],
          },
        },
      },
    })

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(readerStore.exerciseVisible).toBe(false)
  })

  it('toggles maximize button text', async () => {
    const readerStore = useReaderStore()
    readerStore.exerciseVisible = true
    readerStore.currentExerciseHtml = '<html><body>Test</body></html>'

    const wrapper = mount(ReaderExerciseModal, {
      global: {
        stubs: {
          'a-modal': {
            template: '<div v-if="open"><slot /></div>',
            props: ['open'],
          },
        },
      },
    })

    const maximizeButton = wrapper.find('[data-testid="exercise-maximize"]')
    expect(maximizeButton.exists()).toBe(true)
    expect(maximizeButton.attributes('aria-label')).toBe('Maximize')

    await maximizeButton.trigger('click')
    expect(maximizeButton.attributes('aria-label')).toBe('Restore')
  })
})
