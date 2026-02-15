import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent, h } from 'vue'
import { mount } from '@vue/test-utils'
import { useTheme } from '../useTheme'

describe('useTheme', () => {
  let matchMediaMock: any

  beforeEach(() => {
    matchMediaMock = vi.fn().mockImplementation((query) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(), // Deprecated
      removeListener: vi.fn(), // Deprecated
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }))
    vi.stubGlobal('matchMedia', matchMediaMock)

    // Clear classList
    document.documentElement.classList.remove('dark')
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  const TestComponent = defineComponent({
    setup() {
      const theme = useTheme()
      return { ...theme }
    },
    render() {
      return h('div')
    },
  })

  it('should initialize with system theme by default', () => {
    const wrapper = mount(TestComponent)
    expect(wrapper.vm.currentTheme).toBe('system')
  })

  it('should be dark if system is dark and theme is system', () => {
    matchMediaMock.mockImplementation((query: string) => ({
      matches: query === '(prefers-color-scheme: dark)',
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }))

    const wrapper = mount(TestComponent)
    expect(wrapper.vm.isDark).toBe(true)
    expect(document.documentElement.classList.contains('dark')).toBe(true)
  })

  it('should be light if system is light and theme is system', () => {
    matchMediaMock.mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }))

    const wrapper = mount(TestComponent)
    expect(wrapper.vm.isDark).toBe(false)
    expect(document.documentElement.classList.contains('dark')).toBe(false)
  })

  it('should be dark when theme is set to dark, regardless of system', async () => {
    const wrapper = mount(TestComponent)
    wrapper.vm.setTheme('dark')
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.isDark).toBe(true)
    expect(document.documentElement.classList.contains('dark')).toBe(true)
  })

  it('should be light when theme is set to light, regardless of system', async () => {
    // Start with dark system
    matchMediaMock.mockImplementation((query: string) => ({
      matches: query === '(prefers-color-scheme: dark)',
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }))

    const wrapper = mount(TestComponent)
    expect(wrapper.vm.isDark).toBe(true)

    wrapper.vm.setTheme('light')
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.isDark).toBe(false)
    expect(document.documentElement.classList.contains('dark')).toBe(false)
  })

  it('should cleanup listener on unmount', () => {
    const removeEventListener = vi.fn()
    matchMediaMock.mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener,
    }))

    const wrapper = mount(TestComponent)
    wrapper.unmount()
    expect(removeEventListener).toHaveBeenCalledWith('change', expect.any(Function))
  })
})
