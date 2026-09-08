import { mount } from '@vue/test-utils'
import Antd from 'ant-design-vue'
import { describe, expect, it } from 'vitest'
import DictionarySettings from '../DictionarySettings.vue'

describe('DictionarySettings', () => {
  it('仅展示离线保存和候选词数量配置', () => {
    const wrapper = mount(DictionarySettings, {
      props: { saveResultsOffline: false, searchResultLimit: 5 },
      global: { plugins: [Antd] },
    })

    expect(wrapper.text()).toContain('查询候选词个数')
    expect(wrapper.text()).toContain('离线保存查询结果')
    expect(wrapper.text()).not.toContain('手机号')
    expect(wrapper.text()).not.toContain('登录')
    expect(wrapper.getComponent({ name: 'AInputNumber' }).props()).toMatchObject({
      value: 5,
      min: 1,
      max: 15,
    })
  })
})
