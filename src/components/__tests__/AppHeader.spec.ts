import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import Antd from 'ant-design-vue'
import { createPinia } from 'pinia'
import AppHeader from '../AppHeader.vue'
import i18n from '../../i18n'

// Mock Tauri API
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    startDragging: vi.fn(),
    toggleMaximize: vi.fn(),
    isFullscreen: vi.fn(() => Promise.resolve(false)),
  }),
}))

describe('AppHeader.vue', () => {
  it('renders the title prop correctly', () => {
    const title = 'English In Use'
    const wrapper = mount(AppHeader, {
      props: {
        title,
      },
      global: {
        plugins: [createPinia(), i18n, Antd],
      },
    })

    expect(wrapper.text()).toContain(title)
  })

  it('has data-tauri-drag-region attribute', () => {
    const wrapper = mount(AppHeader, {
      props: {
        title: 'Test Title',
      },
      global: {
        plugins: [createPinia(), i18n, Antd],
      },
    })

    expect(wrapper.find('.titlebar').attributes()).toHaveProperty('data-tauri-drag-region')
  })
})
