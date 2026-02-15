import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import AppHeader from '../AppHeader.vue'

// Mock Tauri API
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    startDragging: vi.fn(),
  }),
}))

describe('AppHeader.vue', () => {
  it('renders the title prop correctly', () => {
    const title = 'English In Use'
    const wrapper = mount(AppHeader, {
      props: {
        title,
      },
    })

    expect(wrapper.text()).toContain(title)
  })

  it('has data-tauri-drag-region attribute', () => {
    const wrapper = mount(AppHeader, {
      props: {
        title: 'Test Title',
      },
    })

    expect(wrapper.find('.titlebar').attributes()).toHaveProperty('data-tauri-drag-region')
  })
})
