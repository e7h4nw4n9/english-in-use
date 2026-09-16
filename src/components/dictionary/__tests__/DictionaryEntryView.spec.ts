import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Antd from 'ant-design-vue'
import type { ParsedDictionaryEntry } from '@/features/dictionary/types'
import DictionaryEntryView from '../DictionaryEntryView.vue'

const dictionaryApi = vi.hoisted(() => ({
  getDictionaryAudio: vi.fn(),
}))

vi.mock('@/lib/api', () => dictionaryApi)

const entry: ParsedDictionaryEntry = {
  id: 'patient-id',
  word: 'patient',
  partOfSpeechGroups: [
    {
      key: '0-noun',
      label: 'noun',
      badges: ['A2', 'CET4'],
      inflection: [{ text: 'patientpatients', bold: false, italic: false }],
      pronunciations: [
        {
          region: 'BrE',
          phonetic: 'ˈpeɪʃnt',
          audio: 'patient_gb.mp3',
        },
      ],
      sections: [
        {
          key: 'definitions',
          title: '释义',
          items: [
            {
              id: 'noun-definitions',
              heading: '',
              partsOfSpeech: [],
              pronunciations: [],
              text: '',
              senses: [
                {
                  id: 'noun-sense',
                  sectionTitle: '',
                  marker: '1.',
                  grammar: ['countable'],
                  english: 'a person receiving medical treatment',
                  chinese: '病人',
                  notes: '',
                  examples: [],
                },
              ],
            },
          ],
        },
        {
          key: 'idioms',
          title: '习语',
          items: [
            {
              id: 'patient-idiom',
              heading: 'be patient with sb',
              partsOfSpeech: [],
              pronunciations: [],
              text: '',
              senses: [],
            },
          ],
        },
      ],
    },
    {
      key: '1-adjective',
      label: 'adj.',
      badges: ['B2', 'CET6'],
      inflection: [],
      pronunciations: [],
      sections: [
        {
          key: 'definitions',
          title: '释义',
          items: [
            {
              id: 'adjective-definitions',
              heading: '',
              partsOfSpeech: [],
              pronunciations: [],
              text: '',
              senses: [
                {
                  id: 'adjective-sense',
                  sectionTitle: '',
                  marker: '1.',
                  grammar: [],
                  english: 'able to wait without becoming angry',
                  chinese: '有耐心的',
                  notes: '',
                  examples: [],
                },
              ],
            },
          ],
        },
        {
          key: 'derivatives',
          title: '派生词',
          items: [
            {
              id: 'patiently',
              heading: 'patiently',
              partsOfSpeech: ['adv.'],
              pronunciations: [],
              text: '',
              senses: [
                {
                  id: 'patiently-example',
                  sectionTitle: '',
                  marker: '',
                  grammar: [],
                  english: '',
                  chinese: '',
                  notes: '',
                  examples: [
                    {
                      english: [
                        {
                          text: 'Everyone waited patiently for me to get it right.',
                          bold: false,
                          italic: true,
                        },
                      ],
                      chinese: '大家都耐心地等着我把它搞好。',
                      audios: [],
                    },
                  ],
                },
              ],
            },
          ],
        },
      ],
    },
  ],
}

describe('DictionaryEntryView', () => {
  it('阅读器默认隐藏例句和附加分区，切换不请求 API，新词条重置', async () => {
    const withExamples = structuredClone(entry)
    withExamples.partOfSpeechGroups[0].sections[0].items[0].senses[0].examples = [
      {
        english: [{ text: 'He is a patient.', bold: false, italic: true }],
        chinese: '他是一位病人。',
        audios: [],
      },
    ]
    const wrapper = mount(DictionaryEntryView, {
      props: { entry: withExamples, presentation: 'reader' },
      global: { plugins: [Antd] },
    })
    expect(wrapper.text()).not.toContain('He is a patient.')
    expect(wrapper.text()).not.toContain('be patient with sb')
    await wrapper.get('.examples-toggle').trigger('click')
    expect(wrapper.text()).toContain('He is a patient.')
    expect(wrapper.get('.examples-toggle').text()).toBe('隐藏例句')
    await wrapper.findAll('.ant-tabs-tab')[1].trigger('click')
    await flushPromises()
    expect(wrapper.get('.examples-toggle').text()).toBe('隐藏例句')
    expect(wrapper.text()).not.toContain('Everyone waited patiently')
    await wrapper.setProps({ entry: { ...withExamples, id: 'new-word' } })
    expect(wrapper.text()).not.toContain('He is a patient.')
    expect(wrapper.get('.examples-toggle').text()).toBe('显示例句')
    expect(dictionaryApi.getDictionaryAudio).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('卸载后到达的音频响应不会创建或播放音频', async () => {
    let finish!: (value: unknown) => void
    dictionaryApi.getDictionaryAudio.mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve
      }),
    )
    const createUrl = vi.fn()
    vi.stubGlobal(
      'URL',
      Object.assign(URL, { createObjectURL: createUrl, revokeObjectURL: vi.fn() }),
    )
    const wrapper = mount(DictionaryEntryView, { props: { entry }, global: { plugins: [Antd] } })
    await wrapper.get('.pronunciation').trigger('click')
    wrapper.unmount()
    finish({ bytes: [1], mimeType: 'audio/mpeg' })
    await flushPromises()
    expect(createUrl).not.toHaveBeenCalled()
    vi.unstubAllGlobals()
  })

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('切换词性时同步展示对应标签和释义', async () => {
    const wrapper = mount(DictionaryEntryView, {
      props: { entry },
      global: { plugins: [Antd] },
    })

    expect(wrapper.text()).toContain('A2')
    expect(wrapper.findAll('.part-of-speech-tab-label').map((item) => item.text())).toEqual([
      'n.',
      'adj.',
    ])
    expect(wrapper.get('.entry-header').element.nextElementSibling).toBe(
      wrapper.get('.part-of-speech-tabs').element,
    )
    expect(wrapper.findAll('.entry-content-layout')).toHaveLength(1)
    expect(wrapper.get('.ant-tabs-tabpane-active').text()).toContain(
      'a person receiving medical treatment',
    )
    expect(wrapper.get('.ant-tabs-tabpane-active').text()).not.toContain(
      'able to wait without becoming angry',
    )

    await wrapper.findAll('.ant-tabs-tab')[1].trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('B2')
    expect(wrapper.text()).not.toContain('A2')
    expect(wrapper.get('.ant-tabs-tabpane-active').text()).toContain(
      'able to wait without becoming angry',
    )
    expect(wrapper.get('.ant-tabs-tabpane-active').text()).not.toContain(
      'a person receiving medical treatment',
    )
    expect(wrapper.findAll('.entry-content-layout')).toHaveLength(1)
  })

  it('使用 Tag 展示词性和语法，并为释义保留粗体类和分割结构', () => {
    const wrapper = mount(DictionaryEntryView, {
      props: { entry },
      global: { plugins: [Antd] },
    })

    expect(wrapper.findAll('.part-of-speech-tab-label')).toHaveLength(2)
    expect(wrapper.get('.part-of-speech-tag').text()).toBe('noun')
    expect(wrapper.find('.save-button').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('加入单词本')
    expect(wrapper.get('.grammar-tag').text()).toBe('countable')
    expect(wrapper.findAll('.definition-text')).toHaveLength(2)
    expect(wrapper.findAll('.sense')).toHaveLength(1)
    expect(wrapper.find('.inflection').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('patientpatients')
  })

  it('派生词只有例句时不显示空释义索引', async () => {
    const wrapper = mount(DictionaryEntryView, {
      props: { entry },
      global: { plugins: [Antd] },
    })

    await wrapper.findAll('.ant-tabs-tab')[1].trigger('click')
    await flushPromises()

    expect(wrapper.get('.ant-tabs-tabpane-active').text()).toContain(
      'Everyone waited patiently for me to get it right.',
    )
    expect(wrapper.findAll('.ant-tabs-tabpane-active .sense-number')).toHaveLength(1)
    expect(wrapper.get('.ant-tabs-tabpane-active .example-marker').text()).toBe('◆')
  })

  it('移除释义目录并保留所有正文分区', () => {
    const wrapper = mount(DictionaryEntryView, {
      props: { entry },
      global: { plugins: [Antd] },
    })

    expect(wrapper.find('.entry-directory').exists()).toBe(false)
    expect(wrapper.find('.directory-link').exists()).toBe(false)
    expect(wrapper.get('.ant-tabs-tabpane-active').text()).toContain('be patient with sb')
  })

  it('单词只有一个词性时隐藏 Tab 导航并直接显示正文', () => {
    const singlePartOfSpeechEntry: ParsedDictionaryEntry = {
      ...entry,
      id: 'single-part-of-speech',
      partOfSpeechGroups: [entry.partOfSpeechGroups[0]],
    }
    const wrapper = mount(DictionaryEntryView, {
      props: { entry: singlePartOfSpeechEntry },
      global: { plugins: [Antd] },
    })

    expect(wrapper.get('.part-of-speech-tabs').classes()).toContain('single-part-of-speech')
    expect(wrapper.get('.ant-tabs-tabpane-active').text()).toContain(
      'a person receiving medical treatment',
    )
  })
})
