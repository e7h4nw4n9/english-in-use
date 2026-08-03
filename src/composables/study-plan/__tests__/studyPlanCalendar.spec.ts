import { describe, expect, it } from 'vitest'
import { addDays, addMonths, formatDate, getRange, parseDate } from '../studyPlanCalendar'

describe('studyPlanCalendar', () => {
  it('按本地日期格式化和解析 YYYY-MM-DD', () => {
    const date = new Date(2026, 2, 4)

    expect(formatDate(date)).toBe('2026-03-04')
    expect(parseDate('2026-03-04').getHours()).toBe(0)
  })

  it('计算周视图和月视图范围', () => {
    const anchor = new Date(2026, 2, 4)

    expect(getRange('week', anchor)).toEqual({ start: '2026-03-02', end: '2026-03-08' })
    expect(getRange('month', anchor)).toEqual({ start: '2026-03-01', end: '2026-03-31' })
  })

  it('跨月增减日期', () => {
    const anchor = new Date(2026, 0, 31)

    expect(formatDate(addDays(anchor, 1))).toBe('2026-02-01')
    expect(formatDate(addMonths(anchor, 1))).toBe('2026-02-01')
  })
})
