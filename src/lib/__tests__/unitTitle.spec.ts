import { describe, expect, it } from 'vitest'
import { formatUnitTitle, getUnitPrefix } from '../unitTitle'

describe('单元标题', () => {
  it.each([
    ['RE_00101', 'Later unit', 'Unit 101 Later unit'],
    ['RE_0001', 'Earlier unit', 'Unit 1 Earlier unit'],
    ['RE_00102', 'Appendix 1 Reference', 'Appendix 1 Reference'],
    ['RESOURCE_3', 'Invalid resource', 'Invalid resource'],
    ['RE_12', 'Describing character', 'Unit 12 Describing character'],
    ['RE_12', 'Unit 12 Describing character', 'Unit 12 Describing character'],
    ['RE_12', 'Unit 12', 'Unit 12'],
    ['RE_12', 'unit 12: Character', 'unit 12: Character'],
    ['RE_12', 'Units of measurement', 'Unit 12 Units of measurement'],
    ['RE_0', 'Introduction', 'Introduction'],
    ['RE_-1', 'Introduction', 'Introduction'],
    ['RE_1.5', 'Introduction', 'Introduction'],
    ['RE_1_extra', 'Introduction', 'Introduction'],
    ['RE_1\n', 'Introduction', 'Introduction'],
    ['RE_4294967296', 'Introduction', 'Introduction'],
    ['', 'Introduction', 'Introduction'],
  ])('资源 %s 的标题 %s 显示为 %s', (resourceId, title, expected) => {
    expect(formatUnitTitle(resourceId, title)).toBe(expected)
  })

  it('阅读器已解析的编号优先；同一资源在目录和历史记录中一致', () => {
    expect(getUnitPrefix('section', 'Character', 12)).toBe('Unit 12')
    const prefix = getUnitPrefix('RE_00012', 'Character', 12)
    expect(`${prefix} Character`).toBe(formatUnitTitle('RE_00012', 'Character'))
    expect(getUnitPrefix('RE_12', 'Appendix 1 Reference', 12)).toBe('')
    expect(getUnitPrefix('RE_12', 'Unit 12 Character', 12)).toBe('')
  })
})
