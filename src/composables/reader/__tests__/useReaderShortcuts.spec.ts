import { describe, expect, it, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { mount } from '@vue/test-utils'
import { useReaderShortcuts } from '../useReaderShortcuts'

describe('useReaderShortcuts', () => {
  it('弹窗打开、输入控件获得焦点或事件已被消费时不触发阅读器动作', () => {
    const action = vi.fn()
    let blocked = true
    const Harness = defineComponent({
      setup() {
        useReaderShortcuts({
          isBlocked: () => blocked,
          goBack: action,
          goForward: action,
          togglePlayback: action,
          closeReader: action,
          zoomIn: action,
          zoomOut: action,
          resetZoom: action,
        })
        return () => h('input')
      },
    })
    const wrapper = mount(Harness, { attachTo: document.body })
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    blocked = false
    wrapper.element.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }))
    const consumed = new KeyboardEvent('keydown', { key: 'ArrowLeft', cancelable: true })
    consumed.preventDefault()
    window.dispatchEvent(consumed)
    expect(action).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('binds navigation and playback keys', () => {
    const goBack = vi.fn()
    const goForward = vi.fn()
    const togglePlayback = vi.fn()
    const closeReader = vi.fn()
    const zoomIn = vi.fn()
    const zoomOut = vi.fn()
    const resetZoom = vi.fn()

    const Harness = defineComponent({
      setup() {
        useReaderShortcuts({
          goBack,
          goForward,
          togglePlayback,
          closeReader,
          zoomIn,
          zoomOut,
          resetZoom,
        })
        return () => h('div')
      },
    })

    const wrapper = mount(Harness)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: ' ' }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '-', ctrlKey: true }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '0', ctrlKey: true }))

    expect(goBack).toHaveBeenCalledTimes(1)
    expect(goForward).toHaveBeenCalledTimes(1)
    expect(togglePlayback).toHaveBeenCalledTimes(1)
    expect(closeReader).toHaveBeenCalledTimes(1)
    expect(zoomIn).toHaveBeenCalledTimes(1)
    expect(zoomOut).toHaveBeenCalledTimes(1)
    expect(resetZoom).toHaveBeenCalledTimes(1)

    wrapper.unmount()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }))
    expect(goBack).toHaveBeenCalledTimes(1)
  })
})
