import { describe, expect, it } from 'vitest'
import { getBookDisplayTitle } from '../book'

describe('getBookDisplayTitle', () => {
  it('简称非空时优先返回简称', () => {
    expect(getBookDisplayTitle({ short_title: '  简称  ', title: '完整名称' })).toBe('简称')
  })

  it('简称为空时返回完整名称', () => {
    expect(getBookDisplayTitle({ short_title: '   ', title: '完整名称' })).toBe('完整名称')
    expect(getBookDisplayTitle({ short_title: '\t　\n', title: '完整名称' })).toBe('完整名称')
    expect(getBookDisplayTitle({ short_title: null, title: '完整名称' })).toBe('完整名称')
  })
})
