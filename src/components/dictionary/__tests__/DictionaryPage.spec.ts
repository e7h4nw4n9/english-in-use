import { defineComponent, h } from 'vue'
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ParsedDictionaryTip } from '@/features/dictionary/types'
import DictionaryPage from '../DictionaryPage.vue'

const tip: ParsedDictionaryTip = {
  id: 1,
  wordId: 'fashion-id',
  word: 'fashion',
  pos: 'noun',
  title: 'COLLOCATIONS 词语搭配',
  subtitle: 'Clothes and fashion',
  category: '词语搭配',
  blocks: [{ kind: 'heading1', fragments: [] }],
}

const DictionaryLookupStub = defineComponent({
  emits: ['detail-mode-change'],
  setup(_, { emit }) {
    return () =>
      h('div', { class: 'lookup-stub' }, [
        h('button', { class: 'show-detail', onClick: () => emit('detail-mode-change', true) }, ''),
        h('button', { class: 'show-search', onClick: () => emit('detail-mode-change', false) }, ''),
      ])
  },
})

const DictionaryDiscoveryCardsStub = defineComponent({
  emits: ['open-tip'],
  setup(_, { emit }) {
    return () =>
      h('button', { class: 'discovery-stub', onClick: () => emit('open-tip', tip) }, '精选内容')
  },
})

const DictionaryTipDetailStub = defineComponent({
  props: { tip: { type: Object, required: true } },
  emits: ['back'],
  setup(_, { emit }) {
    return () => h('button', { class: 'tip-detail-stub', onClick: () => emit('back') }, '返回')
  },
})

describe('DictionaryPage', () => {
  it('移除单词本导航，并按查询和贴士详情状态切换内容', async () => {
    const wrapper = mount(DictionaryPage, {
      global: {
        stubs: {
          DictionaryLookup: DictionaryLookupStub,
          DictionaryDiscoveryCards: DictionaryDiscoveryCardsStub,
          DictionaryTipDetail: DictionaryTipDetailStub,
        },
      },
    })

    expect(wrapper.find('.ant-tabs').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('单词本')

    await wrapper.get('.show-detail').trigger('click')
    expect(wrapper.get('.discovery-stub').attributes('style')).toContain('display: none')

    await wrapper.get('.show-search').trigger('click')
    expect(wrapper.get('.discovery-stub').attributes('style')).toBe('')

    await wrapper.get('.discovery-stub').trigger('click')
    expect(wrapper.find('.tip-detail-stub').exists()).toBe(true)
    expect(wrapper.get('.lookup-stub').element.parentElement?.style.display).toBe('none')

    await wrapper.get('.tip-detail-stub').trigger('click')
    expect(wrapper.find('.tip-detail-stub').exists()).toBe(false)
    expect(wrapper.get('.lookup-stub').element.parentElement?.style.display).toBe('')
  })
})
