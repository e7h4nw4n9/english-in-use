import { defineComponent, h } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Antd from 'ant-design-vue'
import DictionaryDiscoveryCards from '../DictionaryDiscoveryCards.vue'

const dictionaryApi = vi.hoisted(() => ({
  getRandomDictionaryGraph: vi.fn(),
  getDictionaryDailyTip: vi.fn(),
  getDictionaryGraphImage: vi.fn(),
  getDictionaryAuthStatus: vi.fn(),
  sendDictionaryVerifyCode: vi.fn(),
  loginDictionary: vi.fn(),
}))

vi.mock('@/lib/api', () => dictionaryApi)

const rawTip = {
  id: 0,
  wordId: 'fashion-id',
  word: 'fashion',
  pos: 'noun',
  unbox: JSON.stringify({
    tile: { type: 'COLLOCATIONS 词语搭配', eng: 'Clothes and fashion' },
    body: [{ h1: [{ tag: 'eng', value: 'Clothes and fashion' }] }],
  }),
}

const GraphPreviewStub = defineComponent({
  props: ['open', 'imageUrl', 'loading', 'error', 'alt'],
  emits: ['close'],
  setup(props) {
    return () => h('div', { class: 'graph-preview-stub', 'data-open': String(props.open) })
  },
})

const LoginModalStub = defineComponent({
  props: ['open'],
  emits: ['cancel', 'success'],
  setup(props) {
    return () => h('div', { class: 'login-modal-stub', 'data-open': String(props.open) })
  },
})

function mountCards() {
  return mount(DictionaryDiscoveryCards, {
    global: {
      plugins: [Antd],
      stubs: {
        DictionaryGraphPreview: GraphPreviewStub,
        DictionaryLoginModal: LoginModalStub,
      },
    },
  })
}

describe('DictionaryDiscoveryCards', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    Object.defineProperty(URL, 'createObjectURL', {
      configurable: true,
      value: vi.fn(() => 'blob:dictionary-image'),
    })
    Object.defineProperty(URL, 'revokeObjectURL', {
      configurable: true,
      value: vi.fn(),
    })
    dictionaryApi.getRandomDictionaryGraph.mockResolvedValue({
      id: 85,
      file: 'img/livingroom.jpg',
      fileSub: 'img_sub/livingroom.jpg',
      english: 'livingroom',
      chinese: '客厅',
    })
    dictionaryApi.getDictionaryDailyTip.mockResolvedValue(rawTip)
    dictionaryApi.getDictionaryGraphImage.mockResolvedValue({
      bytes: [1, 2, 3],
      mimeType: 'image/jpeg',
    })
  })

  it('并行加载两张卡片，并按需打开完整图片和贴士详情', async () => {
    const wrapper = mountCards()
    await flushPromises()

    expect(wrapper.findAll('.feature-card')).toHaveLength(2)
    expect(wrapper.text()).toContain('livingroom')
    expect(wrapper.text()).toContain('fashion')
    expect(dictionaryApi.getDictionaryGraphImage).toHaveBeenCalledWith('img_sub/livingroom.jpg')

    await wrapper.get('.graph-action').trigger('click')
    await flushPromises()
    expect(dictionaryApi.getDictionaryGraphImage).toHaveBeenCalledWith('img/livingroom.jpg')
    expect(wrapper.get('.graph-preview-stub').attributes('data-open')).toBe('true')

    wrapper.getComponent(GraphPreviewStub).vm.$emit('close')
    await wrapper.get('.graph-action').trigger('click')
    await flushPromises()
    expect(dictionaryApi.getDictionaryGraphImage).toHaveBeenCalledTimes(2)

    await wrapper.get('.tip-action').trigger('click')
    expect(wrapper.emitted('open-tip')?.[0]?.[0]).toMatchObject({
      word: 'fashion',
      category: '词语搭配',
    })

    wrapper.unmount()
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:dictionary-image')
  })

  it('原图加载中关闭并重新打开时复用同一个请求', async () => {
    let resolvePreview: ((value: { bytes: number[]; mimeType: string }) => void) | undefined
    dictionaryApi.getDictionaryGraphImage
      .mockResolvedValueOnce({ bytes: [1], mimeType: 'image/jpeg' })
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolvePreview = resolve
          }),
      )
    const wrapper = mountCards()
    await flushPromises()

    await wrapper.get('.graph-action').trigger('click')
    wrapper.getComponent(GraphPreviewStub).vm.$emit('close')
    await wrapper.get('.graph-action').trigger('click')
    expect(dictionaryApi.getDictionaryGraphImage).toHaveBeenCalledTimes(2)

    resolvePreview?.({ bytes: [2], mimeType: 'image/jpeg' })
    await flushPromises()
    expect(wrapper.get('.graph-preview-stub').attributes('data-open')).toBe('true')
  })

  it('鉴权失败时只展示一个登录窗口，登录后重试两项内容', async () => {
    dictionaryApi.getRandomDictionaryGraph
      .mockRejectedValueOnce('[ERR_DICTIONARY_AUTH_REQUIRED] 请重新登录')
      .mockResolvedValueOnce({
        id: 85,
        file: 'img/livingroom.jpg',
        fileSub: 'img_sub/livingroom.jpg',
        english: 'livingroom',
        chinese: '客厅',
      })
    dictionaryApi.getDictionaryDailyTip
      .mockRejectedValueOnce('[ERR_DICTIONARY_AUTH_REQUIRED] 请重新登录')
      .mockResolvedValueOnce(rawTip)
    const wrapper = mountCards()
    await flushPromises()

    const loginModal = wrapper.getComponent(LoginModalStub)
    expect(loginModal.props('open')).toBe(true)
    expect(wrapper.findAll('.login-modal-stub')).toHaveLength(1)

    loginModal.vm.$emit('success')
    await flushPromises()
    expect(dictionaryApi.getRandomDictionaryGraph).toHaveBeenCalledTimes(2)
    expect(dictionaryApi.getDictionaryDailyTip).toHaveBeenCalledTimes(2)
    expect(wrapper.text()).toContain('livingroom')
    expect(wrapper.text()).toContain('fashion')
  })
})
