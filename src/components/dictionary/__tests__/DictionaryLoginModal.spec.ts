import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Antd from 'ant-design-vue'
import DictionaryLoginModal from '../DictionaryLoginModal.vue'

const dictionaryApi = vi.hoisted(() => ({
  getDictionaryAuthStatus: vi.fn(),
  sendDictionaryVerifyCode: vi.fn(),
  loginDictionary: vi.fn(),
}))

vi.mock('@/lib/api', () => dictionaryApi)

describe('DictionaryLoginModal', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    dictionaryApi.getDictionaryAuthStatus.mockResolvedValue({
      authenticated: false,
      cachedPhone: '13800138000',
    })
    dictionaryApi.loginDictionary.mockResolvedValue(undefined)
  })

  it('预填缓存手机号并在登录成功后发送 success', async () => {
    const wrapper = mount(DictionaryLoginModal, {
      props: { open: true },
      attachTo: document.body,
      global: { plugins: [Antd] },
    })
    await flushPromises()

    const inputs = Array.from(document.body.querySelectorAll<HTMLInputElement>('.ant-modal input'))
    expect(inputs[0].value).toBe('13800138000')
    inputs[1].value = '123456'
    inputs[1].dispatchEvent(new Event('input', { bubbles: true }))
    document.body.querySelector<HTMLButtonElement>('.ant-modal .ant-btn-primary')?.click()
    await flushPromises()

    expect(dictionaryApi.loginDictionary).toHaveBeenCalledWith('13800138000', '123456')
    expect(wrapper.emitted('success')).toHaveLength(1)
    wrapper.unmount()
  })
})
