import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ConfigPage from '../ConfigPage.vue'
import * as api from '../../lib/api'
import * as dialog from '@tauri-apps/plugin-dialog'

const startGlobalLoading = vi.fn(() => 1)
const setGlobalLoadingMessage = vi.fn()
const setGlobalLoadingProgress = vi.fn()
const stopGlobalLoading = vi.fn()
const runGlobalLoadingAction = vi.fn(async (action: () => Promise<void>, _message: string) => {
  await action()
  return true
})

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

vi.mock('../../stores/app', () => ({
  useAppStore: () => ({
    globalLoading: false,
    startGlobalLoading,
    setGlobalLoadingMessage,
    setGlobalLoadingProgress,
    stopGlobalLoading,
    runGlobalLoadingAction,
  }),
}))

vi.mock('ant-design-vue', async (importOriginal) => {
  const actual = await importOriginal<any>()
  return {
    ...actual,
    message: {
      config: vi.fn(),
      success: vi.fn(),
      info: vi.fn(),
      warning: vi.fn(),
      error: vi.fn(),
    },
    Modal: {
      confirm: vi.fn((config: any) => config.onOk?.()),
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
  validateLocalBookSource: vi.fn(() =>
    Promise.resolve({
      ok: true,
      warnings: [],
      errors: [],
    }),
  ),
  initializeDatabase: vi.fn(),
  getDefaultSqlitePath: vi.fn(() => Promise.resolve('/mock/path.db')),
  resolveSqlitePath: vi.fn((path: string) => Promise.resolve(path)),
  testDatabaseConnection: vi.fn(),
  testCloudflareGateway: vi.fn(() =>
    Promise.resolve({
      r2: { status: 'Connected' },
      database: { status: 'Connected' },
    }),
  ),
}))

const commonStubs = {
  'a-button': {
    template: '<button class="a-button-stub" @click="$emit(\'click\')"><slot /></button>',
  },
  'a-alert': {
    props: ['message'],
    template: '<div class="a-alert-stub">{{ message }}</div>',
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
  HomeOutlined: true,
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
      props: {
        initialConfig: {
          system: {
            language: 'en',
            theme: 'system',
            log_level: 'info',
            enable_auto_check: true,
            check_interval_mins: 5,
            auto_start_study_timer: false,
          },
          book_source: {
            type: 'Local',
            details: {
              path: '/books',
            },
          },
          database: {
            type: 'SQLite',
            details: {
              path: '/db/test.sqlite',
            },
          },
        } as any,
      },
      global: { stubs: commonStubs },
    })

    const saveBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.saveConfig'))
    await saveBtn?.trigger('click')
    await flushPromises()

    expect(api.saveConfig).toHaveBeenCalled()
    expect(startGlobalLoading).toHaveBeenCalledWith('config.savingConfig')
    expect(setGlobalLoadingMessage).toHaveBeenCalledWith(1, 'config.checkingConnections')
    expect(api.initializeDatabase).toHaveBeenCalled()
    expect(stopGlobalLoading).toHaveBeenCalledWith(1)
    expect(wrapper.emitted('config-saved')).toBeTruthy()
    const backEvents = wrapper.emitted('back')
    expect(backEvents).toBeTruthy()
    expect(backEvents?.[0]?.[0]).toEqual({ reloadHome: true })
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
        auto_start_study_timer: false,
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
    expect(startGlobalLoading).toHaveBeenCalledWith('config.importingConfig')
    expect(setGlobalLoadingMessage).toHaveBeenCalledWith(1, 'config.checkingConnections')
    expect(api.initializeDatabase).toHaveBeenCalled()
    expect(stopGlobalLoading).toHaveBeenCalledWith(1)
    expect(wrapper.emitted('config-imported')).toBeTruthy()
    expect(wrapper.emitted('config-saved')).toBeTruthy()
  })

  it('loads a redacted gateway import into the form without overwriting saved config', async () => {
    const redactedConfig = {
      system: {
        language: 'en',
        theme: 'system',
        log_level: 'info',
        enable_auto_check: true,
        check_interval_mins: 5,
        auto_start_study_timer: false,
      },
      book_source: { type: 'CloudflareGateway', details: {} },
      database: { type: 'CloudflareGateway', details: {} },
      cloudflare_gateway: {
        base_url: 'https://gateway.example.com/',
        access_token: '',
      },
      gateway_configuration_required: true,
    }
    ;(dialog.open as any).mockResolvedValue('/path/to/redacted.toml')
    ;(api.importConfig as any).mockResolvedValue(redactedConfig)

    const wrapper = mount(ConfigPage, {
      global: { stubs: commonStubs },
    })
    const importBtn = wrapper
      .findAll('.a-button-stub')
      .find((button) => button.text().includes('config.importConfig'))
    await importBtn?.trigger('click')
    await flushPromises()

    expect(api.saveConfig).not.toHaveBeenCalled()
    expect(api.initializeDatabase).not.toHaveBeenCalled()
    expect(wrapper.emitted('config-saved')).toBeFalsy()
    expect(wrapper.text()).toContain('config.gatewayMigrationRequired')
  })

  it('auto-fills sqlite path from default when imported config path is empty', async () => {
    const mockConfig = {
      system: {
        language: 'zh',
        theme: 'dark',
        log_level: 'debug',
        enable_auto_check: true,
        check_interval_mins: 10,
        auto_start_study_timer: false,
      },
      book_source: { type: 'Local', details: { path: '/path' } },
      database: { type: 'SQLite', details: { path: '' } },
    }
    ;(dialog.open as any).mockResolvedValue('/path/to/import.toml')
    ;(api.importConfig as any).mockResolvedValue(mockConfig)
    ;(api.getDefaultSqlitePath as any).mockResolvedValue('/auto/sqlite-path')

    const wrapper = mount(ConfigPage, {
      global: { stubs: commonStubs },
    })

    const importBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.importConfig'))
    await importBtn?.trigger('click')
    await flushPromises()

    const savedConfig = (api.saveConfig as any).mock.calls[0]?.[0]
    expect(savedConfig?.database?.details?.path).toBe('/auto/sqlite-path.db')
    expect(api.initializeDatabase).toHaveBeenCalled()
  })

  it('marks back event as reloadHome after importing config', async () => {
    const mockConfig = {
      system: {
        language: 'zh',
        theme: 'dark',
        log_level: 'debug',
        enable_auto_check: true,
        check_interval_mins: 10,
        auto_start_study_timer: false,
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

    await wrapper.find('.back-button').trigger('click')

    const backEvents = wrapper.emitted('back')
    expect(backEvents).toBeTruthy()
    expect(backEvents?.[0]?.[0]).toEqual({ reloadHome: true })
  })

  it('checks R2/D1 and initializes database after successful save', async () => {
    const wrapper = mount(ConfigPage, {
      props: {
        initialConfig: {
          system: {
            language: 'en',
            theme: 'system',
            log_level: 'info',
            enable_auto_check: true,
            check_interval_mins: 5,
            auto_start_study_timer: false,
          },
          book_source: {
            type: 'CloudflareGateway',
            details: {},
          },
          database: {
            type: 'CloudflareGateway',
            details: {},
          },
          cloudflare_gateway: {
            base_url: 'https://gateway.example.com/',
            access_token: 'token',
          },
        } as any,
      },
      global: { stubs: commonStubs },
    })

    const saveBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.saveConfig'))
    await saveBtn?.trigger('click')
    await flushPromises()

    expect(api.saveConfig).toHaveBeenCalled()
    expect(api.testCloudflareGateway).toHaveBeenCalled()
    expect(setGlobalLoadingMessage).toHaveBeenCalledWith(1, 'config.initializingDatabase')
    expect(api.initializeDatabase).toHaveBeenCalled()
  })

  it('ignores an unselected R2 failure when only the gateway database is configured', async () => {
    ;(api.testCloudflareGateway as any).mockResolvedValue({
      r2: { status: 'Disconnected', message: 'unused R2' },
      database: { status: 'Connected' },
    })
    const wrapper = mount(ConfigPage, {
      props: {
        initialConfig: {
          system: {
            language: 'en',
            theme: 'system',
            log_level: 'info',
            enable_auto_check: true,
            check_interval_mins: 5,
            auto_start_study_timer: false,
          },
          book_source: { type: 'Local', details: { path: '/books' } },
          database: { type: 'CloudflareGateway', details: {} },
          cloudflare_gateway: {
            base_url: 'https://gateway.example.com/',
            access_token: 'token',
          },
        } as any,
      },
      global: { stubs: commonStubs },
    })

    const saveBtn = wrapper
      .findAll('.a-button-stub')
      .find((button) => button.text().includes('config.saveConfig'))
    await saveBtn?.trigger('click')
    await flushPromises()

    expect(api.initializeDatabase).toHaveBeenCalled()
    const antd = await import('ant-design-vue')
    expect(antd.message.warning).not.toHaveBeenCalled()
  })

  it('shows warning when cloud connection check fails but import still succeeds', async () => {
    const mockConfig = {
      system: {
        language: 'en',
        theme: 'system',
        log_level: 'info',
        enable_auto_check: true,
        check_interval_mins: 5,
        auto_start_study_timer: false,
      },
      book_source: {
        type: 'CloudflareGateway',
        details: {},
      },
      database: {
        type: 'CloudflareGateway',
        details: {},
      },
      cloudflare_gateway: {
        base_url: 'https://gateway.example.com/',
        access_token: 'token',
      },
    }
    ;(dialog.open as any).mockResolvedValue('/path/to/import.toml')
    ;(api.importConfig as any).mockResolvedValue(mockConfig)
    ;(api.testCloudflareGateway as any).mockRejectedValue(new Error('gateway down'))

    const wrapper = mount(ConfigPage, {
      global: { stubs: commonStubs },
    })

    const importBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.importConfig'))
    await importBtn?.trigger('click')
    await flushPromises()

    const antd = await import('ant-design-vue')
    expect(antd.message.warning).toHaveBeenCalled()
    expect(api.saveConfig).toHaveBeenCalledWith(mockConfig)
    expect(wrapper.emitted('config-imported')).toBeTruthy()
    expect(api.initializeDatabase).not.toHaveBeenCalled()
  })

  it('shows warning when gateway check fails but save still succeeds', async () => {
    ;(api.testCloudflareGateway as any).mockRejectedValue(new Error('gateway down'))

    const wrapper = mount(ConfigPage, {
      props: {
        initialConfig: {
          system: {
            language: 'en',
            theme: 'system',
            log_level: 'info',
            enable_auto_check: true,
            check_interval_mins: 5,
            auto_start_study_timer: false,
          },
          book_source: {
            type: 'Local',
            details: {
              path: '/books',
            },
          },
          database: {
            type: 'CloudflareGateway',
            details: {},
          },
          cloudflare_gateway: {
            base_url: 'https://gateway.example.com/',
            access_token: 'token',
          },
        } as any,
      },
      global: { stubs: commonStubs },
    })

    const saveBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.saveConfig'))
    await saveBtn?.trigger('click')
    await flushPromises()

    const antd = await import('ant-design-vue')
    expect(api.saveConfig).toHaveBeenCalled()
    expect(antd.message.warning).toHaveBeenCalled()
    expect(api.initializeDatabase).not.toHaveBeenCalled()
    expect(wrapper.emitted('config-saved')).toBeTruthy()
  })

  it('auto-falls back to default sqlite path when cloud-synced path is detected on save', async () => {
    ;(api.getDefaultSqlitePath as any).mockResolvedValue('/safe/default.db')

    const wrapper = mount(ConfigPage, {
      props: {
        initialConfig: {
          system: {
            language: 'en',
            theme: 'system',
            log_level: 'info',
            enable_auto_check: true,
            check_interval_mins: 5,
            auto_start_study_timer: false,
          },
          book_source: {
            type: 'Local',
            details: {
              path: '/books',
            },
          },
          database: {
            type: 'SQLite',
            details: {
              path: '/Users/test/OneDrive/work.db',
            },
          },
        } as any,
      },
      global: { stubs: commonStubs },
    })

    const saveBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.saveConfig'))
    await saveBtn?.trigger('click')
    await flushPromises()

    const savedConfig = (api.saveConfig as any).mock.calls[0]?.[0]
    expect(savedConfig?.database?.details?.path).toBe('/safe/default.db')
    expect(api.saveConfig).toHaveBeenCalled()
  })

  it('blocks save when cloud-synced sqlite path fallback fails', async () => {
    ;(api.getDefaultSqlitePath as any).mockResolvedValue('/Users/test/OneDrive/default.db')

    const wrapper = mount(ConfigPage, {
      props: {
        initialConfig: {
          system: {
            language: 'en',
            theme: 'system',
            log_level: 'info',
            enable_auto_check: true,
            check_interval_mins: 5,
            auto_start_study_timer: false,
          },
          book_source: {
            type: 'Local',
            details: {
              path: '/books',
            },
          },
          database: {
            type: 'SQLite',
            details: {
              path: '/Users/test/Google Drive/work.db',
            },
          },
        } as any,
      },
      global: { stubs: commonStubs },
    })

    const saveBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.saveConfig'))
    await saveBtn?.trigger('click')
    await flushPromises()

    const antd = await import('ant-design-vue')
    expect(antd.message.error).toHaveBeenCalled()
    expect(api.saveConfig).not.toHaveBeenCalled()
  })

  it('prevents save when sqlite path is not absolute', async () => {
    const wrapper = mount(ConfigPage, {
      props: {
        initialConfig: {
          system: {
            language: 'en',
            theme: 'system',
            log_level: 'info',
            enable_auto_check: true,
            check_interval_mins: 5,
            auto_start_study_timer: false,
          },
          book_source: {
            type: 'Local',
            details: {
              path: '/books',
            },
          },
          database: {
            type: 'SQLite',
            details: {
              path: 'relative/path',
            },
          },
        } as any,
      },
      global: { stubs: commonStubs },
    })

    const saveBtn = wrapper
      .findAll('.a-button-stub')
      .find((b) => b.text().includes('config.saveConfig'))
    await saveBtn?.trigger('click')
    await flushPromises()

    const antd = await import('ant-design-vue')
    expect(antd.message.error).toHaveBeenCalled()
    expect(api.saveConfig).not.toHaveBeenCalled()
    expect(api.initializeDatabase).not.toHaveBeenCalled()
  })
})
