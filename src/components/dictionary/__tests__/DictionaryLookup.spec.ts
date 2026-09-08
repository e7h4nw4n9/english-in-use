import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Antd from 'ant-design-vue'
import DictionaryLookup from '../DictionaryLookup.vue'
import DictionaryLoginModal from '../DictionaryLoginModal.vue'

const dictionaryApi = vi.hoisted(() => ({
  searchDictionary: vi.fn(),
  getDictionaryWordDetail: vi.fn(),
  getDictionaryAuthStatus: vi.fn(() => Promise.resolve({ authenticated: false })),
  sendDictionaryVerifyCode: vi.fn(),
  loginDictionary: vi.fn(),
  addWordbookEntry: vi.fn(),
  getDictionaryAudio: vi.fn(),
}))

vi.mock('@/lib/api', () => dictionaryApi)

const searchResult = {
  wordId: 'word-id',
  word: 'analogy',
  partsOfSpeech: ['noun'],
  definitionEng: 'a comparison of similar things',
  definitionZh: '类比',
}

function mountLookup() {
  return mount(DictionaryLookup, {
    global: { plugins: [Antd], stubs: { Teleport: true } },
  })
}

describe('DictionaryLookup', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.clearAllMocks()
    dictionaryApi.searchDictionary.mockResolvedValue([searchResult])
    dictionaryApi.getDictionaryWordDetail.mockResolvedValue({
      id: 'word-id',
      word: 'analogy',
      word_body: {},
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('禁用系统输入提示并在停止输入 300ms 后查询', async () => {
    const wrapper = mountLookup()
    const input = wrapper.get('input')

    expect(input.attributes()).toMatchObject({
      autocomplete: 'off',
      autocapitalize: 'none',
      autocorrect: 'off',
      spellcheck: 'false',
    })

    await input.setValue('analogy')
    await vi.advanceTimersByTimeAsync(299)
    expect(dictionaryApi.searchDictionary).not.toHaveBeenCalled()

    await vi.advanceTimersByTimeAsync(1)
    await flushPromises()
    expect(dictionaryApi.searchDictionary).toHaveBeenCalledTimes(1)
    expect(dictionaryApi.searchDictionary).toHaveBeenCalledWith('analogy')
    expect(wrapper.findAll('.result-option')).toHaveLength(1)
    expect(wrapper.get('.search-results').attributes('style')).toContain('background-color')
    expect(wrapper.get('.search-results').attributes('style')).not.toContain('transparent')
  })

  it('回车立即查询并取消等待中的防抖请求', async () => {
    const wrapper = mountLookup()
    const input = wrapper.get('input')
    await input.setValue('analogy')

    await input.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(dictionaryApi.searchDictionary).toHaveBeenCalledTimes(1)

    await vi.advanceTimersByTimeAsync(300)
    expect(dictionaryApi.searchDictionary).toHaveBeenCalledTimes(1)
  })

  it('选择候选后进入二级详情并可返回原查询结果', async () => {
    const wrapper = mountLookup()
    await wrapper.get('input').setValue('analog')
    await vi.advanceTimersByTimeAsync(300)
    await flushPromises()

    await wrapper.get('.result-option').trigger('click')
    await flushPromises()

    const detailEvents = wrapper.emitted('detail-mode-change') || []
    expect(detailEvents[detailEvents.length - 1]).toEqual([true])
    expect(dictionaryApi.getDictionaryWordDetail).toHaveBeenCalledWith('word-id')
    expect(wrapper.get('.dictionary-entry').text()).toContain('analogy')
    expect(wrapper.get('.detail-back').classes()).toContain('operation-button')
    expect(wrapper.find('input').exists()).toBe(false)
    expect(wrapper.find('.search-results').exists()).toBe(false)

    await wrapper.get('.detail-back').trigger('click')
    await flushPromises()

    const searchEvents = wrapper.emitted('detail-mode-change') || []
    expect(searchEvents[searchEvents.length - 1]).toEqual([false])
    expect(wrapper.get('input').element.value).toBe('analog')
    expect(wrapper.findAll('.result-option')).toHaveLength(1)
    expect(dictionaryApi.searchDictionary).toHaveBeenCalledTimes(1)
    expect(dictionaryApi.getDictionaryWordDetail).toHaveBeenCalledTimes(1)
  })

  it('鉴权失败时打开登录窗口并在登录后重试一次', async () => {
    dictionaryApi.searchDictionary
      .mockRejectedValueOnce('[ERR_DICTIONARY_AUTH_REQUIRED] 请重新登录')
      .mockResolvedValueOnce([searchResult])
    const wrapper = mountLookup()
    await wrapper.get('input').setValue('analogy')
    await vi.advanceTimersByTimeAsync(300)
    await flushPromises()

    const loginModal = wrapper.getComponent(DictionaryLoginModal)
    expect(loginModal.props('open')).toBe(true)

    loginModal.vm.$emit('success')
    await flushPromises()
    expect(dictionaryApi.searchDictionary).toHaveBeenCalledTimes(2)
    expect(wrapper.findAll('.result-option')).toHaveLength(1)
  })
})
