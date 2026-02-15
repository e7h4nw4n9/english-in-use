import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import AppFooter from '../AppFooter.vue'
import { useAppStore } from '../../stores/app'

// Mock Tauri API
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}))

// Mock ant-design-vue
vi.mock('ant-design-vue', async (importOriginal) => {
  const actual = await importOriginal<any>()
  return {
    ...actual,
    notification: {
      error: vi.fn(),
    },
  }
})

// Mock vue-i18n
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
  createI18n: () => ({
    global: {
      t: (key: string) => key,
    },
    install: () => {},
  }),
}))

// Mock src/i18n.ts
vi.mock('../../i18n', () => ({
  default: {
    global: {
      t: (key: string) => key,
    },
  },
}))

describe('AppFooter.vue', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('does not render when no active config', () => {
    const store = useAppStore()
    store.connectionStatus = {
      r2: { status: 'NotConfigured' },
      d1: { status: 'NotConfigured' },
    }

    const wrapper = mount(AppFooter, {
      global: {
        stubs: {
          'a-divider': true,
          'a-button': true,
        },
      },
    })
    expect(wrapper.find('.app-footer').exists()).toBe(false)
  })

  it('renders status when R2 is configured', async () => {
    const store = useAppStore()
    store.connectionStatus = {
      r2: { status: 'Connected' },
      d1: { status: 'NotConfigured' },
    }

    const wrapper = mount(AppFooter, {
      global: {
        stubs: {
          'a-divider': true,
          'a-button': true,
        },
      },
    })
    expect(wrapper.find('.app-footer').exists()).toBe(true)
    expect(wrapper.text()).toContain('footer.r2Status')
    expect(wrapper.text()).toContain('footer.connected')
  })

  it('shows error status when disconnected', async () => {
    const store = useAppStore()
    store.connectionStatus = {
      r2: { status: 'Disconnected', message: 'Error message' },
      d1: { status: 'NotConfigured' },
    }

    const wrapper = mount(AppFooter, {
      global: {
        stubs: {
          'a-divider': true,
          'a-button': true,
        },
      },
    })
    expect(wrapper.text()).toContain('footer.disconnected')
  })
})
