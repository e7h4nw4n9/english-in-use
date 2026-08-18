import { describe, expect, it } from 'vitest'
import {
  getDisplayDurationMinutes,
  getTrendAxisLabelPosition,
  getTrendAxisScale,
  getTrendHeight,
} from '../trendAxis'

describe('study stats trend axis', () => {
  it('limits the axis to seven labels with dynamic 10-minute steps', () => {
    expect(getTrendAxisScale(0)).toEqual({
      maxMinutes: 0,
      stepMinutes: 10,
      labels: [0],
    })
    expect(getTrendAxisScale(30)).toEqual({
      maxMinutes: 10,
      stepMinutes: 10,
      labels: [10, 0],
    })
    expect(getTrendAxisScale(57 * 60)).toEqual({
      maxMinutes: 60,
      stepMinutes: 10,
      labels: [60, 50, 40, 30, 20, 10, 0],
    })
    expect(getTrendAxisScale(179 * 60)).toEqual({
      maxMinutes: 180,
      stepMinutes: 30,
      labels: [180, 150, 120, 90, 60, 30, 0],
    })
  })

  it('scales monthly durations above 300 minutes without dense labels', () => {
    expect(getTrendAxisScale(300 * 60)).toEqual({
      maxMinutes: 300,
      stepMinutes: 50,
      labels: [300, 250, 200, 150, 100, 50, 0],
    })
    expect(getTrendAxisScale(301 * 60)).toEqual({
      maxMinutes: 360,
      stepMinutes: 60,
      labels: [360, 300, 240, 180, 120, 60, 0],
    })
    expect(getTrendAxisScale(600 * 60)).toEqual({
      maxMinutes: 600,
      stepMinutes: 100,
      labels: [600, 500, 400, 300, 200, 100, 0],
    })
  })

  it('rounds displayed bar duration up to whole minutes', () => {
    expect(getDisplayDurationMinutes(0)).toBe(0)
    expect(getDisplayDurationMinutes(1)).toBe(1)
    expect(getDisplayDurationMinutes(59)).toBe(1)
    expect(getDisplayDurationMinutes(60)).toBe(1)
    expect(getDisplayDurationMinutes(61)).toBe(2)
    expect(getDisplayDurationMinutes(119)).toBe(2)
    expect(getDisplayDurationMinutes(120)).toBe(2)
  })

  it('positions axis labels against the same scale as the bars', () => {
    expect(getTrendAxisLabelPosition(0, 0)).toBe('0%')
    expect(getTrendAxisLabelPosition(0, 180)).toBe('0%')
    expect(getTrendAxisLabelPosition(90, 180)).toBe('50%')
    expect(getTrendAxisLabelPosition(180, 180)).toBe('100%')
  })

  it('scales bar height against the rounded axis maximum', () => {
    expect(getTrendHeight(0, 30)).toBe('0%')
    expect(getTrendHeight(30 * 60, 30)).toBe('100%')
    expect(Number.parseFloat(getTrendHeight(23 * 60, 30))).toBeCloseTo(76.67, 2)
    expect(getTrendHeight(30, 10)).toBe('5%')
  })
})
