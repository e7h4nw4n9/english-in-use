import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ReaderView from '../ReaderView.vue'
import { createPinia, setActivePinia } from 'pinia'
import { useAppStore } from '../../stores/app'
import { useReaderStore } from '../../stores/reader'
import * as booksApi from '../../lib/api/books'

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}))

// Partial mock for vue-i18n
vi.mock('vue-i18n', async (importOriginal) => {
  const actual = await importOriginal<typeof import('vue-i18n')>()
  return {
    ...actual,
    useI18n: () => ({
      t: (key: string) => key,
      locale: { value: 'en' },
    }),
  }
})

// Mock the API
vi.mock('../../lib/api/books', () => ({
  getBookMetadata: vi.fn(() =>
    Promise.resolve({
      toc: [
        {
          title: 'Unit 1',
          key: 'RE_0001',
          startPage: '12',
          endPage: '14',
          audioFiles: [{ path: 'unit1.mp3', title: 'Unit 1 Audio' }],
          children: [
            {
              title: 'Section 1.1',
              key: 'RE_0001_1',
              startPage: '12',
              endPage: '12',
              audioFiles: [{ path: 'sec1.mp3', title: 'Section 1.1 Audio' }],
            },
            {
              title: 'Section 1.2',
              key: 'RE_0001_2',
              startPage: '13',
              endPage: '13',
              // No audio here, should fallback to Unit 1
            },
          ],
        },
      ],
      pages: {
        '12': { label: '12', image_path: 'page12.jpg' },
        '13': { label: '13', image_path: 'page13.jpg' },
        '14': { label: '14', image_path: 'page14.jpg' },
      },
      pageLabels: ['12', '13', '14'],
      pageWidth: 1000,
      pageHeight: 1400,
    }),
  ),
  resolvePageResource: vi.fn((_code, label) =>
    Promise.resolve(`asset://localhost/path/to/page${label}.jpg`),
  ),
  resolveBookAsset: vi.fn((_code, path) => Promise.resolve(`asset://localhost/${path}`)),
  resolveExerciseResource: vi.fn(() => Promise.resolve('exercise.html')),
  getExerciseHtml: vi.fn(() =>
    Promise.resolve({
      html: '<html><body>Exercise</body></html>',
      url: 'eiuasset://localhost/exercise.html',
    }),
  ),
  getReadingProgress: vi.fn(() => Promise.resolve(null)),
  updateReadingProgress: vi.fn(() => Promise.resolve()),
}))

// Mock ResizeObserver
global.ResizeObserver = class ResizeObserver {
  callback: any
  constructor(callback: any) {
    this.callback = callback
  }
  observe() {}
  unobserve() {}
  disconnect() {}
  // Helper to trigger callback in tests
  trigger(entries: any[]) {
    this.callback(entries)
  }
}

// Mock Ant Design Vue components that might be complex
vi.mock('ant-design-vue', async () => {
  const actual = await vi.importActual('ant-design-vue')
  return {
    ...(actual as any),
  }
})

describe('ReaderView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.mocked(booksApi.getExerciseHtml).mockResolvedValue({
      html: '<html><body>Exercise</body></html>',
      url: 'eiuasset://localhost/exercise.html',
    })
    const appStore = useAppStore()
    appStore.currentBook = {
      id: 1,
      book_group: 2,
      product_code: 'essgiuebk',
      title: 'Test Book',
      author: 'Author',
      product_type: 'imgbook',
      cover: 'cover.jpg',
      sort_num: 1,
    }
    appStore.config = {
      system: {
        language: 'en',
        theme: 'system',
        log_level: 'info',
        enable_debug_tools: true,
        enable_auto_check: false,
        check_interval_mins: 5,
      },
      book_source: null,
      database: null,
    }
  })

  it('renders correctly and loads metadata', async () => {
    mount(ReaderView, {
      global: {
        stubs: {
          LeftOutlined: true,
          RightOutlined: true,
          FullscreenOutlined: true,
          MenuFoldOutlined: true,
          MenuUnfoldOutlined: true,
          ZoomInOutlined: true,
          ZoomOutOutlined: true,
          BlockOutlined: true,
          FileTextOutlined: true,
          ArrowLeftOutlined: true,
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-radio-group': true,
          'a-radio-button': true,
          'a-button-group': true,
          'a-input-search': true,
          'a-tree': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    const readerStore = useReaderStore()
    expect(readerStore.currentPageLabel).toBe('12')
  })

  it('updates currentPageAudioFiles when page changes', async () => {
    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          CustomerServiceOutlined: true,
          AudioOutlined: true,
          UnorderedListOutlined: true,
          'a-popover': {
            template: '<div><slot name="title" /><slot name="content" /><slot /></div>',
          },
        },
      },
    })

    // Wait for metadata to load
    await new Promise((resolve) => setTimeout(resolve, 50))

    const readerStore = useReaderStore()

    // Test page 12 (should match Section 1.1)
    readerStore.currentPageLabel = '12'
    await wrapper.vm.$nextTick()
    const vm = wrapper.vm as any
    expect(vm.currentPageAudioFiles).toHaveLength(1)
    expect(vm.currentPageAudioFiles[0].path).toBe('sec1.mp3')

    // Test page 13 (should fallback to Unit 1 since Section 1.2 has no audio)
    readerStore.currentPageLabel = '13'
    await wrapper.vm.$nextTick()
    expect(vm.currentPageAudioFiles).toHaveLength(1)
    expect(vm.currentPageAudioFiles[0].path).toBe('unit1.mp3')

    // Test a page outside range
    readerStore.currentPageLabel = '99'
    await wrapper.vm.$nextTick()
    expect(vm.currentPageAudioFiles).toHaveLength(0)
  })

  it('shows debug modal when debugVisible is true', async () => {
    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          BugOutlined: true,
          'a-modal': {
            template: '<div class="debug-modal"><slot /></div>',
            props: ['open'],
          },
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 50))

    const readerStore = useReaderStore()
    readerStore.debugVisible = true
    await wrapper.vm.$nextTick()

    expect(wrapper.findComponent({ name: 'ReaderDebugModal' }).exists()).toBe(true)
  })

  it('hides debug modal and resets debugVisible when debug setting is disabled', async () => {
    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          BugOutlined: true,
          'a-modal': {
            template: '<div class="debug-modal"><slot /></div>',
            props: ['open'],
          },
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 50))

    const appStore = useAppStore()
    const readerStore = useReaderStore()
    readerStore.debugVisible = true
    appStore.config = {
      system: {
        language: 'en',
        theme: 'system',
        log_level: 'info',
        enable_debug_tools: false,
        enable_auto_check: false,
        check_interval_mins: 5,
      },
      book_source: null,
      database: null,
    }

    await wrapper.vm.$nextTick()

    expect(readerStore.debugVisible).toBe(false)
    expect(wrapper.findComponent({ name: 'ReaderDebugModal' }).exists()).toBe(false)
  })

  it('handles zoom keyboard shortcuts', async () => {
    mount(ReaderView, {
      global: {
        stubs: {
          LeftOutlined: true,
          RightOutlined: true,
          FullscreenOutlined: true,
          MenuFoldOutlined: true,
          MenuUnfoldOutlined: true,
          ZoomInOutlined: true,
          ZoomOutOutlined: true,
          BlockOutlined: true,
          FileTextOutlined: true,
          ArrowLeftOutlined: true,
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-radio-group': true,
          'a-radio-button': true,
          'a-button-group': true,
          'a-input-search': true,
          'a-tree': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    const readerStore = useReaderStore()
    readerStore.zoomLevel = 1.0

    // Zoom in (Ctrl + =)
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true }))
    expect(readerStore.zoomLevel).toBeGreaterThan(1.0)

    // Zoom out (Ctrl + -)
    const currentZoom = readerStore.zoomLevel
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '-', ctrlKey: true }))
    expect(readerStore.zoomLevel).toBeLessThan(currentZoom)

    // Reset zoom (Ctrl + 0)
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '0', ctrlKey: true }))
    expect(readerStore.zoomLevel).toBe(1.0)
  })

  it('handles arrow key navigation', async () => {
    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          LeftOutlined: true,
          RightOutlined: true,
          FullscreenOutlined: true,
          MenuFoldOutlined: true,
          MenuUnfoldOutlined: true,
          ZoomInOutlined: true,
          ZoomOutOutlined: true,
          BlockOutlined: true,
          FileTextOutlined: true,
          ArrowLeftOutlined: true,
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-radio-group': true,
          'a-radio-button': true,
          'a-button-group': true,
          'a-input-search': true,
          'a-tree': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    // Wait for metadata to load
    await new Promise((resolve) => setTimeout(resolve, 100))

    const readerStore = useReaderStore()
    readerStore.currentPageLabel = '13' // Middle page
    await wrapper.vm.$nextTick()

    // ArrowRight -> Forward
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }))
    await wrapper.vm.$nextTick()
    expect(readerStore.currentPageLabel).toBe('14')

    // ArrowLeft -> Backward
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }))
    await wrapper.vm.$nextTick()
    expect(readerStore.currentPageLabel).toBe('13')
  })

  it('stops audio playback and closes player when page changes', async () => {
    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          LeftOutlined: true,
          RightOutlined: true,
          FullscreenOutlined: true,
          MenuFoldOutlined: true,
          MenuUnfoldOutlined: true,
          ZoomInOutlined: true,
          ZoomOutOutlined: true,
          BlockOutlined: true,
          FileTextOutlined: true,
          ArrowLeftOutlined: true,
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-radio-group': true,
          'a-radio-button': true,
          'a-button-group': true,
          'a-input-search': true,
          'a-tree': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 50))

    const readerStore = useReaderStore()
    readerStore.currentAudioPath = 'unit1.mp3'
    readerStore.isPlaying = true
    readerStore.audioCurrentTime = 12
    readerStore.audioDuration = 99

    readerStore.currentPageLabel = '13'
    await wrapper.vm.$nextTick()

    expect(readerStore.currentAudioPath).toBeNull()
    expect(readerStore.isPlaying).toBe(false)
    expect(readerStore.audioCurrentTime).toBe(0)
    expect(readerStore.audioDuration).toBe(0)
  })

  it('keeps audio playback state when switching spread to single mode', async () => {
    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          LeftOutlined: true,
          RightOutlined: true,
          FullscreenOutlined: true,
          MenuFoldOutlined: true,
          MenuUnfoldOutlined: true,
          ZoomInOutlined: true,
          ZoomOutOutlined: true,
          BlockOutlined: true,
          FileTextOutlined: true,
          ArrowLeftOutlined: true,
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-radio-group': true,
          'a-radio-button': true,
          'a-button-group': true,
          'a-input-search': true,
          'a-tree': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 50))

    const readerStore = useReaderStore()
    readerStore.viewMode = 'spread'
    await wrapper.vm.$nextTick()

    readerStore.currentAudioPath = 'unit1.mp3'
    readerStore.isPlaying = true
    readerStore.audioCurrentTime = 18
    readerStore.audioDuration = 120

    readerStore.viewMode = 'single'
    await wrapper.vm.$nextTick()

    expect(readerStore.currentAudioPath).toBe('unit1.mp3')
    expect(readerStore.isPlaying).toBe(true)
    expect(readerStore.audioCurrentTime).toBe(18)
    expect(readerStore.audioDuration).toBe(120)
  })

  it('continues pending audio load when switching spread to single mode', async () => {
    let resolveAudioAsset: (value: string) => void = () => {}
    const pendingAudioAsset = new Promise<string>((resolve) => {
      resolveAudioAsset = resolve
    })
    vi.mocked(booksApi.resolveBookAsset).mockReturnValueOnce(pendingAudioAsset)

    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          LeftOutlined: true,
          RightOutlined: true,
          FullscreenOutlined: true,
          MenuFoldOutlined: true,
          MenuUnfoldOutlined: true,
          ZoomInOutlined: true,
          ZoomOutOutlined: true,
          BlockOutlined: true,
          FileTextOutlined: true,
          ArrowLeftOutlined: true,
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-radio-group': true,
          'a-radio-button': true,
          'a-button-group': true,
          'a-input-search': true,
          'a-tree': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 50))

    const readerStore = useReaderStore()
    readerStore.viewMode = 'spread'
    await wrapper.vm.$nextTick()

    const canvas = wrapper.findComponent({ name: 'ReaderCanvas' })
    ;(canvas.vm as any).$emit('overlayClick', {
      type: 'audio',
      audio: { path: 'unit1.mp3' },
    })
    await wrapper.vm.$nextTick()

    readerStore.viewMode = 'single'
    await wrapper.vm.$nextTick()

    resolveAudioAsset('asset://localhost/unit1.mp3')
    await flushPromises()

    expect(readerStore.currentAudioPath).toBe('unit1.mp3')
    expect(booksApi.resolveBookAsset).toHaveBeenCalledWith('essgiuebk', 'unit1.mp3')
  })

  it('opens exercise modal when an exercise overlay is clicked', async () => {
    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          'a-modal': {
            template: '<div class="a-modal-stub"><slot /></div>',
            props: ['open'],
          },
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    // Wait for metadata to load
    await new Promise((resolve) => setTimeout(resolve, 50))

    const readerStore = useReaderStore()
    readerStore.currentPageLabel = '12'

    // Mock metadata with exercise overlay
    const vm = wrapper.vm as any
    vm.metadata.pages['12'].overlays = [
      {
        x: 10,
        y: 10,
        w: 50,
        h: 50,
        type: 'exercise',
        exercise: { name: 'Practice 1', resource_id: 'p1' },
      },
    ]
    await wrapper.vm.$nextTick()

    const canvas = wrapper.findComponent({ name: 'ReaderCanvas' })
    ;(canvas.vm as any).$emit('overlayClick', vm.metadata.pages['12'].overlays[0])
    await flushPromises()

    expect(readerStore.exerciseVisible).toBe(true)
    expect(readerStore.currentExerciseTitle).toBe('Practice 1')
  })

  it('opens exercise modal when a learning-object overlay is clicked', async () => {
    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          'a-modal': {
            template: '<div class="a-modal-stub"><slot /></div>',
            props: ['open'],
          },
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 50))

    const readerStore = useReaderStore()
    readerStore.currentPageLabel = '12'

    // Mock metadata with learning-object overlay
    const vm = wrapper.vm as any
    vm.metadata.pages['12'].overlays = [
      {
        x: 10,
        y: 10,
        w: 50,
        h: 50,
        type: 'learning-object',
        exercise: { name: 'Learning Obj 1', resource_id: 'lo1' },
      },
    ]
    await wrapper.vm.$nextTick()

    const canvas = wrapper.findComponent({ name: 'ReaderCanvas' })
    ;(canvas.vm as any).$emit('overlayClick', vm.metadata.pages['12'].overlays[0])
    await flushPromises()

    expect(readerStore.exerciseVisible).toBe(true)
    expect(readerStore.currentExerciseTitle).toBe('Learning Obj 1')
  })

  it('shows global loading and waits for exercise download before opening modal', async () => {
    let resolveExerciseHtml: (value: booksApi.ExerciseHtmlResponse) => void = () => {}
    const pendingHtml = new Promise<booksApi.ExerciseHtmlResponse>((resolve) => {
      resolveExerciseHtml = resolve
    })
    vi.mocked(booksApi.getExerciseHtml).mockReturnValueOnce(pendingHtml)

    const wrapper = mount(ReaderView, {
      global: {
        stubs: {
          'a-modal': {
            template: '<div class="a-modal-stub"><slot /></div>',
            props: ['open'],
          },
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 50))

    const appStore = useAppStore()
    const readerStore = useReaderStore()
    readerStore.currentPageLabel = '12'

    const vm = wrapper.vm as any
    vm.metadata.pages['12'].overlays = [
      {
        x: 10,
        y: 10,
        w: 50,
        h: 50,
        type: 'exercise',
        exercise: { name: 'Practice Pending', resource_id: 'pending' },
      },
    ]
    await wrapper.vm.$nextTick()

    const canvas = wrapper.findComponent({ name: 'ReaderCanvas' })
    ;(canvas.vm as any).$emit('overlayClick', vm.metadata.pages['12'].overlays[0])
    await wrapper.vm.$nextTick()

    expect(appStore.globalLoading).toBe(true)
    expect(readerStore.exerciseVisible).toBe(false)

    resolveExerciseHtml({
      html: '<html><body>Ready</body></html>',
      url: 'eiuasset://localhost/pending.html',
    })
    await flushPromises()

    expect(appStore.globalLoading).toBe(false)
    expect(readerStore.exerciseVisible).toBe(true)
    expect(readerStore.currentExerciseTitle).toBe('Practice Pending')
  })

  it('switches to single view mode when container width is small', async () => {
    const readerStore = useReaderStore()
    readerStore.viewMode = 'spread'

    // Create a wrapper to get access to the ResizeObserver instance
    mount(ReaderView, {
      global: {
        stubs: {
          LeftOutlined: true,
          RightOutlined: true,
          FullscreenOutlined: true,
          MenuFoldOutlined: true,
          MenuUnfoldOutlined: true,
          ZoomInOutlined: true,
          ZoomOutOutlined: true,
          BlockOutlined: true,
          FileTextOutlined: true,
          ArrowLeftOutlined: true,
          'a-button': true,
          'a-tooltip': true,
          'a-divider': true,
          'a-radio-group': true,
          'a-radio-button': true,
          'a-button-group': true,
          'a-input-search': true,
          'a-tree': true,
          'a-spin': true,
          'a-slider': true,
        },
      },
    })
  })
})
