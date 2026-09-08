import { defineComponent, h } from 'vue'
import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'
import Antd from 'ant-design-vue'
import DictionaryGraphPreview from '../DictionaryGraphPreview.vue'

const ModalStub = defineComponent({
  name: 'AModal',
  props: {
    open: Boolean,
    getContainer: Function,
    mask: Boolean,
    closable: Boolean,
  },
  setup(_props, { slots }) {
    return () => h('div', { class: 'modal-stub' }, slots.default?.())
  },
})

function mountPreview() {
  const contentContainer = document.createElement('main')
  contentContainer.className = 'app-main-container'
  const mountPoint = document.createElement('div')
  contentContainer.appendChild(mountPoint)
  document.body.appendChild(contentContainer)

  const wrapper = mount(DictionaryGraphPreview, {
    attachTo: mountPoint,
    props: {
      open: true,
      imageUrl: 'blob:dictionary-graph',
      loading: false,
      error: '',
      alt: 'livingroom 客厅',
    },
    global: {
      plugins: [Antd],
      stubs: { AModal: ModalStub },
    },
  })

  return { wrapper, contentContainer }
}

/** 创建包含两个触点的触摸事件。
 * @param type - 触摸事件类型。
 * @param distance - 两个触点之间的横向距离。
 */
function createPinchEvent(type: string, distance: number) {
  const event = new Event(type, { bubbles: true, cancelable: true })
  Object.defineProperty(event, 'touches', {
    value: [
      { clientX: 0, clientY: 0 },
      { clientX: distance, clientY: 0 },
    ],
  })
  return event
}

/** 创建用于测试拖拽的指针事件。
 * @param type - 指针事件类型。
 * @param options - 指针类型、按钮和坐标。
 */
function createPointerEvent(
  type: string,
  options: {
    pointerId?: number
    pointerType?: string
    button?: number
    clientX?: number
    clientY?: number
  } = {},
) {
  const event = new MouseEvent(type, {
    bubbles: true,
    cancelable: true,
    button: options.button ?? 0,
    clientX: options.clientX ?? 0,
    clientY: options.clientY ?? 0,
  })
  Object.defineProperties(event, {
    pointerId: { value: options.pointerId ?? 1 },
    pointerType: { value: options.pointerType ?? 'mouse' },
  })
  return event
}

/** 为滚动区域设置可测试的尺寸。
 * @param element - 图片滚动区域。
 */
function setOverflowSize(element: Element) {
  Object.defineProperties(element, {
    clientWidth: { configurable: true, value: 500 },
    clientHeight: { configurable: true, value: 400 },
    scrollWidth: { configurable: true, value: 1000 },
    scrollHeight: { configurable: true, value: 800 },
    setPointerCapture: { configurable: true, value: () => undefined },
    hasPointerCapture: { configurable: true, value: () => true },
    releasePointerCapture: { configurable: true, value: () => undefined },
  })
}

describe('DictionaryGraphPreview', () => {
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('将预览挂载到标题栏下方的内容区域且不使用全视口遮罩', () => {
    const { wrapper, contentContainer } = mountPreview()
    const modal = wrapper.getComponent(ModalStub)
    const getContainer = modal.props('getContainer')

    expect(modal.props('mask')).toBe(false)
    expect(modal.props('closable')).toBe(false)
    expect(getContainer).toBeDefined()
    expect(getContainer?.()).toBe(contentContainer)
    expect(wrapper.get('img').attributes()).toMatchObject({
      src: 'blob:dictionary-graph',
      alt: 'livingroom 客厅',
    })
  })

  it('点击顶部左侧返回按钮时发送 close 事件', async () => {
    const { wrapper } = mountPreview()

    expect(wrapper.get('.preview-toolbar').element.nextElementSibling).toBe(
      wrapper.get('.preview-content').element,
    )
    expect(wrapper.get('.preview-back').text()).toContain('返回')
    expect(wrapper.get('.preview-back').attributes('aria-label')).toBe('返回图解词汇')

    await wrapper.get('.preview-back').trigger('click')

    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('默认以内容宽度显示图片并支持按钮缩放和恢复', async () => {
    const { wrapper } = mountPreview()
    const image = wrapper.get('img')
    const percentage = wrapper.get('.preview-zoom-percentage')

    expect(image.attributes('style')).toContain('width: 100%')
    expect(percentage.text()).toBe('100%')

    await wrapper.get('[aria-label="放大图片"]').trigger('click')
    expect(image.attributes('style')).toContain('width: 125%')
    expect(percentage.text()).toBe('125%')

    await wrapper.get('[aria-label="缩小图片"]').trigger('click')
    expect(image.attributes('style')).toContain('width: 100%')

    await wrapper.get('[aria-label="放大图片"]').trigger('click')
    await wrapper.get('[aria-label="恢复图片大小和位置"]').trigger('click')
    expect(image.attributes('style')).toContain('width: 100%')
    expect(percentage.text()).toBe('100%')
  })

  it('仅使用 Ctrl 或 Command 组合键时响应滚轮缩放', async () => {
    const { wrapper } = mountPreview()
    const content = wrapper.get('.preview-content')

    content.element.dispatchEvent(
      new WheelEvent('wheel', { deltaY: -100, bubbles: true, cancelable: true }),
    )
    expect(wrapper.get('.preview-zoom-percentage').text()).toBe('100%')

    content.element.dispatchEvent(
      new WheelEvent('wheel', {
        deltaY: -100,
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    )
    await wrapper.vm.$nextTick()
    expect(wrapper.get('.preview-zoom-percentage').text()).toBe('110%')

    content.element.dispatchEvent(
      new WheelEvent('wheel', {
        deltaY: 100,
        metaKey: true,
        bubbles: true,
        cancelable: true,
      }),
    )
    await wrapper.vm.$nextTick()
    expect(wrapper.get('.preview-zoom-percentage').text()).toBe('100%')
  })

  it('支持双指缩放并在切换图片时恢复默认比例', async () => {
    const { wrapper } = mountPreview()
    const content = wrapper.get('.preview-content')

    content.element.dispatchEvent(createPinchEvent('touchstart', 100))
    content.element.dispatchEvent(createPinchEvent('touchmove', 150))
    await wrapper.vm.$nextTick()
    expect(wrapper.get('.preview-zoom-percentage').text()).toBe('150%')

    await wrapper.setProps({ imageUrl: 'blob:another-dictionary-graph' })
    expect(wrapper.get('.preview-zoom-percentage').text()).toBe('100%')
  })

  it('支持使用鼠标拖动溢出的图片显示位置', async () => {
    const { wrapper } = mountPreview()
    const content = wrapper.get('.preview-content')
    setOverflowSize(content.element)
    content.element.scrollLeft = 120
    content.element.scrollTop = 80

    content.element.dispatchEvent(
      createPointerEvent('pointerdown', { pointerId: 7, clientX: 200, clientY: 160 }),
    )
    content.element.dispatchEvent(
      createPointerEvent('pointermove', { pointerId: 7, clientX: 150, clientY: 120 }),
    )
    await wrapper.vm.$nextTick()

    expect(content.classes()).toContain('is-dragging')
    expect(content.element.scrollLeft).toBe(170)
    expect(content.element.scrollTop).toBe(120)

    content.element.dispatchEvent(createPointerEvent('pointerup', { pointerId: 7 }))
    content.element.dispatchEvent(
      createPointerEvent('pointermove', { pointerId: 7, clientX: 100, clientY: 80 }),
    )
    await wrapper.vm.$nextTick()
    expect(content.classes()).not.toContain('is-dragging')
    expect(content.element.scrollLeft).toBe(170)
    expect(content.element.scrollTop).toBe(120)
  })

  it('触摸指针和非左键不会进入自定义拖拽', async () => {
    const { wrapper } = mountPreview()
    const content = wrapper.get('.preview-content')
    setOverflowSize(content.element)

    content.element.dispatchEvent(
      createPointerEvent('pointerdown', { pointerType: 'touch', clientX: 100, clientY: 100 }),
    )
    content.element.dispatchEvent(
      createPointerEvent('pointerdown', { button: 2, clientX: 100, clientY: 100 }),
    )
    await wrapper.vm.$nextTick()

    expect(content.classes()).not.toContain('is-dragging')
    expect(content.element.scrollLeft).toBe(0)
    expect(content.element.scrollTop).toBe(0)
  })

  it('恢复操作同时重置缩放比例和图片位置', async () => {
    const { wrapper } = mountPreview()
    const content = wrapper.get('.preview-content')
    setOverflowSize(content.element)
    content.element.scrollLeft = 140
    content.element.scrollTop = 90

    expect(wrapper.get('.preview-zoom-percentage').text()).toBe('100%')
    expect(wrapper.get('[aria-label="恢复图片大小和位置"]').attributes('disabled')).toBeUndefined()
    await wrapper.get('[aria-label="恢复图片大小和位置"]').trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.get('.preview-zoom-percentage').text()).toBe('100%')
    expect(content.element.scrollLeft).toBe(0)
    expect(content.element.scrollTop).toBe(0)
  })
})
