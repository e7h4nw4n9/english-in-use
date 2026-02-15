import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ConfigPage from '../ConfigPage.vue'
import * as api from '../../lib/api'

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
      success: vi.fn(),
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
  'a-menu': { template: '<div class="a-menu-stub"><slot /></div>' },
  'a-menu-item': {
    template: '<div class="a-menu-item-stub" @click="$emit(\'click\')"><slot /></div>',
  },
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

describe('ConfigPage.vue', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders initial state correctly', async () => {
    const wrapper = mount(ConfigPage, {
      global: {
        stubs: commonStubs,
      },
    })

    expect(wrapper.find('.config-title-text').text()).toContain('config.title')
    expect(wrapper.find('.system-settings-stub').exists()).toBe(true)
  })

  it('calls saveConfig when handleSave is called', async () => {
    ;(api.saveConfig as any).mockResolvedValue(undefined)
    const wrapper = mount(ConfigPage, {
      global: {
        stubs: commonStubs,
      },
    })

    const saveButtons = wrapper
      .findAll('.a-button-stub')
      .filter((b) => b.text().includes('config.saveConfig'))
    await saveButtons[0].trigger('click')

    expect(api.saveConfig).toHaveBeenCalled()
  })

  it('emits back event when back button is clicked', async () => {
    const wrapper = mount(ConfigPage, {
      props: {
        allowBack: true,
      },
      global: {
        stubs: commonStubs,
      },
    })

    const backButton = wrapper.find('.back-button')
    await backButton.trigger('click')

    expect(wrapper.emitted()).toHaveProperty('back')
  })
})
