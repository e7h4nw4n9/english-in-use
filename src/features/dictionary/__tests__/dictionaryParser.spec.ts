import { describe, expect, it } from 'vitest'
import { parseDictionaryEntry } from '../dictionaryParser'

describe('parseDictionaryEntry', () => {
  it('解析词头、音标、释义、例句及音频', () => {
    const entry = parseDictionaryEntry({
      id: 'word-id',
      word: 'analogy',
      word_body: {
        top_data: {
          h: [
            { value: 'ana·logy' },
            { value: '[Ox5000 key_L][CEFR_C1_M]' },
            { value: '[CET6][NETM]' },
          ],
          pos: [{ value: 'noun' }],
          prongs: [{ geo: 'BrE', phon: 'əˈnælədʒi', audio: 'analogy#_gb_2' }],
          top_text: [{ value: '(pl. analogies)', font_Italic: 1 }],
        },
        sngs_data: [
          {
            sngs_data: {
              sn_g: [
                {
                  id: 'sense-1',
                  sng_text: [{ value: '1.' }, { value: '[countable]' }],
                  def_eng: [{ value: ' a comparison of similar things' }],
                  def_simp: [{ value: '类比' }],
                  x_gs: [
                    {
                      x_eng: [{ value: 'by analogy', bold: 1 }],
                      x_simp: [{ value: '用类推法' }],
                      xaudio: [{ value: { geo: 'br', url: '_analogy#_gbs_1' } }],
                    },
                  ],
                },
              ],
            },
          },
        ],
      },
    })

    expect(entry.word).toBe('ana·logy')
    const noun = entry.partOfSpeechGroups[0]
    expect(noun.label).toBe('noun')
    expect(noun.badges).toEqual(['C1', 'CET6', 'NETM'])
    expect(noun.pronunciations[0].audio).toBe('analogy#_gb_2')
    const senses = noun.sections[0].items[0].senses
    expect(senses[0]).toMatchObject({
      marker: '1.',
      grammar: ['countable'],
      english: 'a comparison of similar things',
      chinese: '类比',
    })
    expect(senses[0].examples[0].audios[0].name).toBe('_analogy#_gbs_1')
  })

  it('按词性分组释义并使用当前词性的 CEFR 标签', () => {
    const entry = parseDictionaryEntry({
      id: 'patient-id',
      word_body: {
        top_data: {
          h: [
            { value: 'patient' },
            { value: '[Ox3000 key_L][CEFR_B2_L]' },
            { value: '[CET4][CET6][NETM]' },
          ],
          prongs: [{ geo: 'BrE', phon: 'ˈpeɪʃnt', audio: 'patient#_gb_2' }],
        },
        sngs_data: [
          {
            sngs_data: {
              pos: [{ value: 'noun' }],
              subentry_cefr: [{ value: '[Ox3000 key_L][CEFR_A2_S]' }],
              sn_g: [
                {
                  id: 'noun-sense',
                  sng_text: [{ value: '1.[Ox3000 key_S][CEFR_A2_S]' }],
                  def_eng: [{ value: 'a person receiving medical treatment' }],
                  def_simp: [{ value: '病人' }],
                },
              ],
            },
          },
          {
            sngs_data: {
              pos: [{ value: 'adj.' }],
              subentry_cefr: [{ value: '[Ox3000 key_L][CEFR_B2_S]' }],
              sn_g: [
                {
                  id: 'adjective-sense',
                  sng_text: [{ value: '1.[Ox3000 key_S][CEFR_B2_S]' }],
                  def_eng: [{ value: 'able to wait without becoming angry' }],
                  def_simp: [{ value: '有耐心的' }],
                },
              ],
            },
          },
        ],
      },
    })

    expect(entry.partOfSpeechGroups.map((group) => group.label)).toEqual(['noun', 'adj.'])
    expect(entry.partOfSpeechGroups[0].badges).toEqual(['A2', 'CET4', 'CET6', 'NETM'])
    expect(entry.partOfSpeechGroups[1].badges).toEqual(['B2', 'CET4', 'CET6', 'NETM'])
    expect(JSON.stringify(entry)).not.toMatch(/Oxford|Ox3000|Ox5000/i)
  })

  it('忽略结构化 unbox 内容并保留纯文本用法提示', () => {
    const structuredUnbox = [
      {
        id: 'unbox-id',
        tile: { type: 'SYNONYMS 同义词' },
        body: [{ ul: [{ li: [{ value: '[diomond]stain' }] }] }],
      },
    ]
    const entry = parseDictionaryEntry({
      id: 'mark-id',
      word_body: {
        top_data: { h: [{ value: 'mark' }], pos: [{ value: 'noun' }] },
        sngs_data: [
          {
            sngs_data: {
              pos: [{ value: 'noun' }],
              sn_g: [
                {
                  id: 'mark-sense',
                  def_eng: [{ value: 'a small area of dirt' }],
                  un: [{ value: '普通用法提示' }],
                  unbox: structuredUnbox,
                },
              ],
              unbox: structuredUnbox,
            },
          },
        ],
      },
    })

    const group = entry.partOfSpeechGroups[0]
    expect(group.sections[0].items[0].senses[0].notes).toBe('普通用法提示')
    expect(group.sections.map((section) => section.key)).not.toContain('usage')
    expect(JSON.stringify(entry)).not.toContain('[diomond]')
  })

  it('按目录结构拆分习语、派生词和补充内容', () => {
    const entry = parseDictionaryEntry({
      id: 'along-id',
      word_body: {
        top_data: { h: [{ value: 'along' }] },
        sngs_data: [
          {
            sngs_data: {
              pos: [{ value: 'adv.' }],
              sn_g: [
                {
                  id: 'definition',
                  def_eng: [{ value: 'forward' }],
                  def_simp: [{ value: '向前' }],
                },
              ],
              idm_gs: [
                {
                  idm_g: [
                    {
                      id: 'idiom',
                      idm_name: [{ value: 'along with sb[Ox3000 key_S] [CEFR_B1_S]' }],
                      sn_g: [
                        {
                          id: 'idiom-sense',
                          def_eng: [{ value: 'in addition to somebody' }],
                          def_simp: [{ value: '除某人以外' }],
                        },
                      ],
                    },
                  ],
                },
              ],
              dr_gs: [
                {
                  id: 'derivative',
                  top_g: {
                    h: [{ value: 'alongside' }],
                    pos: [{ value: 'adv.' }],
                    prongs: [{ geo: 'BrE', phon: 'əˌlɒŋˈsaɪd', audio: 'alongside#_gb_1' }],
                  },
                  sn_gs: {
                    sn_g: [
                      {
                        id: 'derivative-sense',
                        def_eng: [{ value: 'next to something' }],
                        def_simp: [{ value: '在旁边' }],
                      },
                    ],
                  },
                },
              ],
              unbox: [{ value: '用法提示' }],
              xrgs: [{ value: '相关词条' }],
            },
          },
        ],
      },
    })

    const sections = entry.partOfSpeechGroups[0].sections
    expect(sections.map((section) => section.key)).toEqual([
      'definitions',
      'idioms',
      'derivatives',
      'usage',
      'related',
    ])
    expect(sections[1].items[0].heading).toBe('along with sb')
    expect(sections[2].items[0]).toMatchObject({
      heading: 'alongside',
      partsOfSpeech: ['adv.'],
      pronunciations: [{ region: 'BrE', phonetic: 'əˌlɒŋˈsaɪd', audio: 'alongside#_gb_1' }],
    })
    expect(sections[3].items[0].text).toBe('用法提示')
    expect(sections[4].items[0].text).toBe('相关词条')
    expect(JSON.stringify(entry)).not.toMatch(/Oxford|Ox3000|Ox5000/i)
  })
})
