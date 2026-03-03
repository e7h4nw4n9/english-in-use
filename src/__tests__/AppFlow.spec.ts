import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import App from '../App.vue'
import * as api from '../lib/api'
import { listen } from '@tauri-apps/api/event'
import { useAppStore } from '../stores/app'
import { useReaderStore } from '../stores/reader'

// Mock dependencies
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}))

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    startDragging: vi.fn(),
  }),
}))

vi.mock('@tauri-apps/plugin-log', () => ({
  info: vi.fn(),
  error: vi.fn(),
  debug: vi.fn(),
  warn: vi.fn(),
}))

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
    locale: { value: 'en' },
  }),
  createI18n: () => ({
    global: {
      t: (key: string) => key,
    },
    install: () => {},
  }),
}))

vi.mock('../i18n', () => ({
  default: {
    global: {
      t: (key: string) => key,
    },
  },
}))

vi.mock('../lib/api', () => ({
  loadConfig: vi.fn(),
  checkConnectionStatus: vi.fn(),
  initializeDatabase: vi.fn(),
  getBooks: vi.fn(() => Promise.resolve([])),
  getBookCover: vi.fn(),
  bytesToImageUrl: vi.fn(),
  getDefaultSqlitePath: vi.fn(() => Promise.resolve('/mock/path')),
  resolveSqlitePath: vi.fn((path: string) => Promise.resolve(path)),
}))

// Global mock for matchMedia
vi.stubGlobal(
  'matchMedia',
  vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  })),
)

const commonStubs = {
  'a-config-provider': { template: '<div><slot /></div>' },
  'a-spin': { props: ['tip'], template: '<div class="a-spin-stub">{{tip}}</div>' },
  'a-progress': { props: ['percent'], template: '<div class="a-progress-stub">{{percent}}</div>' },
  AppHeader: { props: ['title'], template: '<div class="header-stub">{{title}}</div>' },
  AppFooter: { template: '<div class="footer-stub" />' },
  ConfigPage: { template: '<div class="config-page-stub" />' },
  BookList: { template: '<div class="book-list-stub" />' },
  ReaderView: { template: '<div class="reader-view-stub" />' },
}

describe('App Flow Integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('shows ConfigPage when app initializes with invalid config', async () => {
    // Mock invalid config
    ;(api.loadConfig as any).mockResolvedValue({
      system: { language: 'en', theme: 'system', enable_auto_check: false },
      book_source: null,
      database: null,
    })

    const wrapper = mount(App, {
      global: { stubs: commonStubs },
    })

    // Should show loading spinner initially
    expect(wrapper.find('.a-spin-stub').exists()).toBe(true)

    // Wait for Store.initApp and all watches
    await flushPromises()

    // Check state directly
    const store = useAppStore()
    expect(store.isLoading).toBe(false)
    expect(store.isConfigValid).toBe(false)

    // Wait for the watch(isConfigValid) to trigger showConfig.value = true
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.config-page-stub').exists()).toBe(true)
    expect(wrapper.find('.header-stub').text()).toBe('config.title')
  })

  it('shows BookList when app initializes with valid config', async () => {
    // Mock valid config
    ;(api.loadConfig as any).mockResolvedValue({
      system: { language: 'en', theme: 'system', enable_auto_check: true },
      book_source: { type: 'Local', details: { path: '/books' } },
      database: { type: 'SQLite', details: { path: '/db' } },
    })
    ;(api.initializeDatabase as any).mockResolvedValue(false)
    ;(api.checkConnectionStatus as any).mockResolvedValue({
      r2: { status: 'NotConfigured' },
      d1: { status: 'Connected' },
    })

    const wrapper = mount(App, {
      global: { stubs: commonStubs },
    })

    await flushPromises()
    await wrapper.vm.$nextTick()

    // Should switch to BookList
    expect(wrapper.find('.book-list-stub').exists()).toBe(true)
    expect(wrapper.find('.header-stub').text()).toBe('app.title')
  })

  it('switches to ConfigPage when open-settings event is received', async () => {
    let openSettingsCallback: any
    ;(listen as any).mockImplementation((event: string, cb: any) => {
      if (event === 'open-settings') {
        openSettingsCallback = cb
      }
      return Promise.resolve(() => {})
    })

    // Start with valid config -> BookList
    ;(api.loadConfig as any).mockResolvedValue({
      system: { language: 'en', theme: 'system', enable_auto_check: true },
      book_source: { type: 'Local', details: { path: '/books' } },
      database: { type: 'SQLite', details: { path: '/db' } },
    })

    const wrapper = mount(App, {
      global: { stubs: commonStubs },
    })

    await flushPromises()
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.book-list-stub').exists()).toBe(true)

    // Trigger open-settings
    openSettingsCallback()
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.config-page-stub').exists()).toBe(true)
  })

  it('keeps header visible when opening ConfigPage from reader with hidden reader UI', async () => {
    let openSettingsCallback: any
    ;(listen as any).mockImplementation((event: string, cb: any) => {
      if (event === 'open-settings') {
        openSettingsCallback = cb
      }
      return Promise.resolve(() => {})
    })
    ;(api.loadConfig as any).mockResolvedValue({
      system: { language: 'en', theme: 'system', enable_auto_check: true },
      book_source: { type: 'Local', details: { path: '/books' } },
      database: { type: 'SQLite', details: { path: '/db' } },
    })

    const wrapper = mount(App, {
      global: { stubs: commonStubs },
    })

    await flushPromises()
    await wrapper.vm.$nextTick()

    const appStore = useAppStore()
    const readerStore = useReaderStore()

    appStore.currentBook = { title: 'Test Reader Book' } as any
    readerStore.isUiVisible = false
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.header-stub').attributes('style')).toContain('display: none')

    openSettingsCallback()
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.config-page-stub').exists()).toBe(true)
    expect(wrapper.find('.header-stub').isVisible()).toBe(true)
    expect(wrapper.find('.header-stub').text()).toBe('config.title')
  })

  it('completes the full cycle: invalid config -> save valid config -> back to BookList', async () => {
    // 1. Start with invalid config, then refresh to valid config after back
    ;(api.loadConfig as any)
      .mockResolvedValueOnce({
        system: { language: 'en', theme: 'system', enable_auto_check: false },
        book_source: null,
        database: null,
      })
      .mockResolvedValueOnce({
        system: { language: 'en', theme: 'system', enable_auto_check: true },
        book_source: { type: 'Local', details: { path: '/books' } },
        database: { type: 'SQLite', details: { path: '/db' } },
      })

    const wrapper = mount(App, {
      global: {
        stubs: {
          ...commonStubs,
          ConfigPage: {
            props: ['initialConfig'],
            template: `
              <div class="config-page-stub">
                <button class="save-btn" @click="$emit('config-saved')">Save</button>
                <button class="back-btn" @click="$emit('back', { reloadHome: true })">Back</button>
              </div>
            `,
          },
        },
      },
    })

    await flushPromises()
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.config-page-stub').exists()).toBe(true)

    // 2. Save config marks pending reload, but stays on config page until user goes back
    await wrapper.find('.save-btn').trigger('click')
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.config-page-stub').exists()).toBe(true)

    // 3. Back to home should refresh config and show BookList
    await wrapper.find('.back-btn').trigger('click')
    await flushPromises()
    await wrapper.vm.$nextTick()

    expect(api.loadConfig).toHaveBeenCalledTimes(2)
    expect(wrapper.find('.book-list-stub').exists()).toBe(true)
    expect(wrapper.find('.config-page-stub').exists()).toBe(false)
  })

  it('reloads home with imported config after leaving ConfigPage', async () => {
    let openSettingsCallback: any
    ;(listen as any).mockImplementation((event: string, cb: any) => {
      if (event === 'open-settings') {
        openSettingsCallback = cb
      }
      return Promise.resolve(() => {})
    })
    ;(api.loadConfig as any)
      .mockResolvedValueOnce({
        system: { language: 'en', theme: 'system', enable_auto_check: false },
        book_source: { type: 'Local', details: { path: '/books/a' } },
        database: { type: 'SQLite', details: { path: '/db/a' } },
      })
      .mockResolvedValueOnce({
        system: { language: 'en', theme: 'system', enable_auto_check: false },
        book_source: { type: 'Local', details: { path: '/books/b' } },
        database: { type: 'SQLite', details: { path: '/db/b' } },
      })
    ;(api.initializeDatabase as any).mockResolvedValue(false)

    const wrapper = mount(App, {
      global: {
        stubs: {
          ...commonStubs,
          ConfigPage: {
            template: `
              <div class="config-page-stub">
                <button class="import-btn" @click="$emit('config-imported')">Import</button>
                <button class="back-btn" @click="$emit('back', { reloadHome: true })">Back</button>
              </div>
            `,
          },
        },
      },
    })

    await flushPromises()
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.book-list-stub').exists()).toBe(true)

    openSettingsCallback()
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.config-page-stub').exists()).toBe(true)

    await wrapper.find('.import-btn').trigger('click')
    await wrapper.find('.back-btn').trigger('click')
    await flushPromises()
    await wrapper.vm.$nextTick()

    expect(api.loadConfig).toHaveBeenCalledTimes(2)
    expect(wrapper.find('.book-list-stub').exists()).toBe(true)
    expect(wrapper.find('.config-page-stub').exists()).toBe(false)
  })
})
