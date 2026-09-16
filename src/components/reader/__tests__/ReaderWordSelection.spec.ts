import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent } from 'vue'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { Button } from 'ant-design-vue'
import ReaderWordSelection from '../ReaderWordSelection.vue'

const api = vi.hoisted(() => ({ recognizeReaderPage: vi.fn(), copyReaderWord: vi.fn() }))
vi.mock('@/lib/api/readerOcr', async (original) => ({
  ...(await original<typeof import('@/lib/api/readerOcr')>()),
  ...api,
}))
const result = {
  width: 1000,
  height: 1000,
  lines: [
    {
      text: 'patient',
      words: [{ text: 'patient', rect: { x: 0.1, y: 0.2, width: 0.2, height: 0.05 } }],
    },
  ],
}
let wrapper: VueWrapper

/** 测试环境的 PointerEvent 是 MouseEvent 替身，显式定义只读指针字段。 */
function pointer(element: Element, type: string, fields: Record<string, unknown> = {}) {
  const event = new Event(type, { bubbles: true, cancelable: true })
  for (const [key, value] of Object.entries(fields)) Object.defineProperty(event, key, { value })
  element.dispatchEvent(event)
}

/** 创建有真实父容器及图片矩形的选词测试环境。 */
async function setup(hotspot = false) {
  const Harness = defineComponent({
    components: { ReaderWordSelection },
    data: () => ({
      pages: [{ label: '1', url: 'page.jpg' }],
      disabled: false,
      zoom: 1,
      metadata: {
        pageWidth: 1000,
        pageHeight: 1000,
        pages: { '1': { overlays: hotspot ? [{ x: 100, y: 200, w: 200, h: 50 }] : [] } },
      },
    }),
    template: `<div><div class="page-surface" data-page-label="1"><img src="page.jpg"></div><ReaderWordSelection product-code="book" :pages="pages" :metadata="metadata" :zoom="zoom" :disabled="disabled" /></div>`,
  })
  wrapper = mount(Harness, { attachTo: document.body, global: { plugins: [Button] } })
  const image = wrapper.get('img').element as HTMLImageElement
  Object.defineProperty(image, 'complete', { value: true, configurable: true })
  Object.defineProperty(image, 'naturalWidth', { value: 1000, configurable: true })
  image.getBoundingClientRect = () => ({
    x: 0,
    y: 0,
    left: 0,
    top: 0,
    right: 1000,
    bottom: 1000,
    width: 1000,
    height: 1000,
    toJSON() {},
  })
  wrapper.element.getBoundingClientRect = image.getBoundingClientRect
  await wrapper.get('img').trigger('load')
  await flushPromises()
}

describe('阅读器选词交互', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    api.recognizeReaderPage.mockReset().mockResolvedValue(result)
    api.copyReaderWord.mockReset().mockResolvedValue(undefined)
  })
  afterEach(() => {
    wrapper?.unmount()
    vi.useRealTimers()
  })

  it('双击选词、复制并清理；重复选择不重复识别', async () => {
    await setup()
    await wrapper.get('img').trigger('dblclick', { clientX: 150, clientY: 225 })
    expect(wrapper.get('.reader-word-actions').text()).toBe('复制查询')
    await wrapper.findAll('button')[0].trigger('click')
    await flushPromises()
    expect(api.copyReaderWord).toHaveBeenCalledWith('patient')
    expect(wrapper.find('.reader-word-actions').exists()).toBe(false)
    await wrapper.get('img').trigger('dblclick', { clientX: 150, clientY: 225 })
    expect(api.recognizeReaderPage).toHaveBeenCalledTimes(1)
  })

  it('热点即使没有可见标记也不能选词', async () => {
    await setup(true)
    await wrapper.get('img').trigger('dblclick', { clientX: 150, clientY: 225 })
    expect(wrapper.find('.reader-word-actions').exists()).toBe(false)
  })

  it.each(['touch', 'pen'])('%s 长按 450ms 才选词', async (pointerType) => {
    await setup()
    pointer(wrapper.get('img').element, 'pointerdown', {
      pointerType,
      pointerId: 1,
      isPrimary: true,
      clientX: 150,
      clientY: 225,
    })
    await vi.advanceTimersByTimeAsync(449)
    expect(wrapper.find('.reader-word-actions').exists()).toBe(false)
    await vi.advanceTimersByTimeAsync(1)
    expect(wrapper.find('.reader-word-actions').exists()).toBe(true)
    pointer(wrapper.get('img').element, 'pointerup')
    await wrapper.findAll('button')[1].trigger('click')
    expect(wrapper.getComponent(ReaderWordSelection).emitted('query')).toEqual([['patient']])
  })

  it('移动、第二触点及取消事件都会取消长按', async () => {
    await setup()
    const image = wrapper.get('img')
    for (const reason of ['pointermove', 'pointerdown', 'pointercancel']) {
      pointer(image.element, 'pointerdown', {
        pointerType: 'touch',
        pointerId: 1,
        isPrimary: true,
        clientX: 150,
        clientY: 225,
      })
      pointer(image.element, reason, {
        pointerType: 'touch',
        pointerId: 2,
        isPrimary: false,
        clientX: 170,
        clientY: 225,
      })
      await vi.advanceTimersByTimeAsync(500)
      expect(wrapper.find('.reader-word-actions').exists()).toBe(false)
    }
  })

  it('滚动与 Escape 清理选区，Escape 不继续传播', async () => {
    await setup()
    await wrapper.get('img').trigger('dblclick', { clientX: 150, clientY: 225 })
    await wrapper.trigger('scroll')
    expect(wrapper.find('.reader-word-actions').exists()).toBe(false)
    await wrapper.get('img').trigger('dblclick', { clientX: 150, clientY: 225 })
    const listener = vi.fn()
    window.addEventListener('keydown', listener)
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }))
    await flushPromises()
    window.removeEventListener('keydown', listener)
    expect(listener).not.toHaveBeenCalled()
    expect(wrapper.find('.reader-word-actions').exists()).toBe(false)
  })

  it('翻页时旧识别结果不会进入新页面', async () => {
    let resolve!: (value: typeof result) => void
    api.recognizeReaderPage.mockReturnValueOnce(
      new Promise((done) => {
        resolve = done
      }),
    )
    await setup()
    await wrapper.setData({ pages: [{ label: '2', url: 'page2.jpg' }] })
    resolve(result)
    await flushPromises()
    await wrapper.get('img').trigger('dblclick', { clientX: 150, clientY: 225 })
    expect(wrapper.find('.reader-word-actions').exists()).toBe(false)
  })
})
