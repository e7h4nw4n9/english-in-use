import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import Antd from 'ant-design-vue'
import DictionaryTipDetail from '../DictionaryTipDetail.vue'

describe('DictionaryTipDetail', () => {
  it('渲染完整标题、双语正文和返回操作', async () => {
    const wrapper = mount(DictionaryTipDetail, {
      props: {
        tip: {
          id: 0,
          wordId: 'fashion-id',
          word: 'fashion',
          pos: 'noun',
          title: 'COLLOCATIONS 词语搭配',
          subtitle: 'Clothes and fashion',
          category: '词语搭配',
          blocks: [
            {
              kind: 'heading1' as const,
              fragments: [
                {
                  text: 'Clothes and fashion 服装与时尚',
                  bold: false,
                  italic: false,
                  chinese: false,
                  breakAfter: false,
                },
              ],
            },
            {
              kind: 'listItem' as const,
              fragments: [
                {
                  text: 'be wearing',
                  bold: true,
                  italic: false,
                  chinese: false,
                  breakAfter: false,
                },
              ],
            },
          ],
        },
      },
      global: { plugins: [Antd] },
    })

    expect(wrapper.get('.tip-toolbar').text()).toContain('COLLOCATIONS 词语搭配')
    expect(wrapper.get('.tip-heading1').text()).toContain('Clothes and fashion')
    expect(wrapper.get('.tip-listItem .bold').text()).toBe('be wearing')

    await wrapper.get('.tip-back').trigger('click')
    expect(wrapper.emitted('back')).toHaveLength(1)
  })
})
