import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ConfigPage from '../ConfigPage.vue'
import * as api from '../../lib/api'
import * as dialog from '@tauri-apps/plugin-dialog'

// Mock matchMedia globally
vi.stubGlobal(
  'matchMedia',
  vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
)

// Mock dependencies
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
    locale: { value: 'en' },
  }),
}))

vi.mock('../composables/useTheme', () => ({
  useTheme: () => ({
    setTheme: vi.fn(),
  }),
}))

vi.mock('ant-design-vue', async (importOriginal) => {
  const actual = await importOriginal<any>()
  return {
    ...actual,
    message: {
      config: vi.fn(),
      success: vi.fn(),
      error: vi.fn(),
    },
    Modal: {
      success: vi.fn((config: any) => config.onOk?.()),
    },
    theme: {
      useToken: () => ({
        token: {
          colorBgContainer: '#fff',
          colorText: '#000',
          colorBorderSecondary: '#eee',
          colorTextSecondary: '#666',
        },
      }),
    },
  }
})

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-log', () => ({
  info: vi.fn(),
  error: vi.fn(),
  debug: vi.fn(),
  warn: vi.fn(),
}))

vi.mock('../../lib/api', () => ({
  saveConfig: vi.fn(),
  exportConfig: vi.fn(),
  importConfig: vi.fn(),
  getDefaultSqlitePath: vi.fn(() => Promise.resolve('/mock/path.db')),
  testDatabaseConnection: vi.fn(),
  testR2Connection: vi.fn(),
}))

vi.mock('../../lib/api/system', () => ({
  restartApp: vi.fn(),
}))

const commonStubs = {
  'a-button': {
    template: '<button class="a-button-stub" @click="$emit(\'click\')"><slot /></button>',
  },
  'a-menu': { template: '<div><slot /></div>' },
  'a-menu-item': { template: '<div><slot /></div>' },
  SystemSettings: { template: '<div class="system-settings-stub" />' },
  BookSourceSettings: { template: '<div class="book-source-settings-stub" />' },
  DatabaseSettings: { template: '<div class="database-settings-stub" />' },
  SettingOutlined: true,
  BookOutlined: true,
  DatabaseOutlined: true,
  ArrowLeftOutlined: true,
  DownloadOutlined: true,
  UploadOutlined: true,
}

describe('ConfigPage.vue Core Logic', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders initial state correctly', async () => {
    const wrapper = mount(ConfigPage, {
      global: { stubs: commonStubs },
    })
    expect(wrapper.find('.config-title-text').text()).toContain('config.title')
  })

  it('triggers save flow', async () => {
    const wrapper = mount(ConfigPage, {
      global: { stubs: commonStubs },
    })

    const saveBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.saveConfig'))
    await saveBtn?.trigger('click')
    await flushPromises()

    expect(api.saveConfig).toHaveBeenCalled()
    const system = await import('../../lib/api/system')
    expect(system.restartApp).toHaveBeenCalled()
  })

  it('triggers export flow', async () => {
    ;(dialog.save as any).mockResolvedValue('/path/to/export.toml')
    const wrapper = mount(ConfigPage, {
      global: { stubs: commonStubs },
    })

    const exportBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.exportConfig'))
    await exportBtn?.trigger('click')
    await flushPromises()

    expect(dialog.save).toHaveBeenCalled()
    expect(api.exportConfig).toHaveBeenCalled()
  })

  it('triggers import flow', async () => {
    const mockConfig = {
      system: {
        language: 'zh',
        theme: 'dark',
        log_level: 'debug',
        enable_auto_check: true,
        check_interval_mins: 10,
      },
      book_source: { type: 'Local', details: { path: '/path' } },
      database: { type: 'SQLite', details: { path: '/db' } },
    }
    ;(dialog.open as any).mockResolvedValue('/path/to/import.toml')
    ;(api.importConfig as any).mockResolvedValue(mockConfig)

    const wrapper = mount(ConfigPage, {
      global: { stubs: commonStubs },
    })

    const importBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.importConfig'))
    await importBtn?.trigger('click')
    await flushPromises()

    expect(dialog.open).toHaveBeenCalled()
    expect(api.importConfig).toHaveBeenCalledWith('/path/to/import.toml')
    expect(api.saveConfig).toHaveBeenCalledWith(mockConfig)
  })
})
