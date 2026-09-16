import { describe, expect, it } from 'vitest'
import { findReaderWord, type RecognizedPage } from '../readerOcr'

const page: RecognizedPage = {
  width: 1000,
  height: 1400,
  lines: [
    {
      text: "can't wait",
      words: [
        { text: "can't", rect: { x: 0.1, y: 0.2, width: 0.1, height: 0.02 } },
        { text: 'wait', rect: { x: 0.25, y: 0.2, width: 0.1, height: 0.02 } },
      ],
    },
  ],
}

describe('阅读器单词命中', () => {
  it('按归一化坐标返回单个单词并保留撇号', () => {
    expect(findReaderWord(page, 0.15, 0.21, [])?.text).toBe("can't")
    expect(findReaderWord(page, 0.3, 0.21, [])?.text).toBe('wait')
    expect(findReaderWord(page, 0.23, 0.21, [])).toBeUndefined()
  })
  it('热点内部和与热点相交的整个单词均不能选择', () => {
    const hotspots = [{ x: 0.19, y: 0.2, width: 0.04, height: 0.03 }]
    expect(findReaderWord(page, 0.195, 0.21, hotspots)).toBeUndefined()
    expect(findReaderWord(page, 0.15, 0.21, hotspots)).toBeUndefined()
    expect(findReaderWord(page, 0.3, 0.21, hotspots)?.text).toBe('wait')
  })
})
