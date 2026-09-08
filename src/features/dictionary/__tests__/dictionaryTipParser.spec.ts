import { describe, expect, it } from 'vitest'
import { parseDictionaryTip } from '../dictionaryTipParser'

describe('parseDictionaryTip', () => {
  it('解析标题、分类和带样式的双语内容块', () => {
    const result = parseDictionaryTip({
      id: 0,
      wordId: 'fashion-id',
      word: 'fashion',
      pos: 'noun',
      unbox: JSON.stringify({
        tile: {
          type: 'COLLOCATIONS 词语搭配',
          eng: ' Clothes and fashion',
          simp: ' 服装与时尚',
        },
        body: [
          {
            h1: [
              { tag: 'eng', value: 'Clothes and fashion', bold: 0 },
              { tag: 'simp', value: ' 服装与时尚', bold: 0 },
            ],
          },
          {
            ul: [
              {
                li: [
                  { tag: 'eb', value: 'be wearing', bold: 1 },
                  { tag: 'eng', value: ' a new outfit', bold: 0 },
                  { tag: 'simp', value: ' 穿着一身新衣裳', bold: 0 },
                  { tag: 'geo', value: ' BrE', font_Italic: '1' },
                ],
              },
            ],
          },
        ],
      }),
    })

    expect(result.title).toBe('COLLOCATIONS 词语搭配')
    expect(result.subtitle).toBe('Clothes and fashion')
    expect(result.category).toBe('词语搭配')
    expect(result.blocks.map((block) => block.kind)).toEqual(['heading1', 'listItem'])
    expect(result.blocks[1].fragments[0].bold).toBe(true)
    expect(result.blocks[1].fragments[2].chinese).toBe(true)
    expect(result.blocks[1].fragments[3].italic).toBe(true)
  })

  it('拒绝无效或缺少正文的 unbox 内容', () => {
    expect(() =>
      parseDictionaryTip({ id: 0, wordId: 'id', word: 'word', pos: '', unbox: 'invalid' }),
    ).toThrow('实用贴士内容格式错误')
    expect(() =>
      parseDictionaryTip({
        id: 0,
        wordId: 'id',
        word: 'word',
        pos: '',
        unbox: JSON.stringify({ tile: { type: 'TYPE 类型', eng: 'Title' }, body: [] }),
      }),
    ).toThrow('实用贴士内容不完整')
  })
})
