const MINUTE_SECONDS = 60
const MIN_AXIS_STEP_MINUTES = 10
const MAX_AXIS_INTERVALS = 6

export interface TrendAxisScale {
  maxMinutes: number
  stepMinutes: number
  labels: number[]
}

/** 根据最长学习时长生成最多七个刻度的 Y 轴模型。
 * @param maxDurationSeconds - 当前趋势中的最长学习秒数。
 */
export function getTrendAxisScale(maxDurationSeconds: number): TrendAxisScale {
  const safeDuration = Math.max(0, maxDurationSeconds)
  if (safeDuration === 0) {
    return {
      maxMinutes: 0,
      stepMinutes: MIN_AXIS_STEP_MINUTES,
      labels: [0],
    }
  }

  const durationMinutes = safeDuration / MINUTE_SECONDS
  const stepMinutes =
    Math.ceil(durationMinutes / MAX_AXIS_INTERVALS / MIN_AXIS_STEP_MINUTES) * MIN_AXIS_STEP_MINUTES
  const maxMinutes = Math.ceil(durationMinutes / stepMinutes) * stepMinutes
  const labels: number[] = []

  for (let value = maxMinutes; value >= 0; value -= stepMinutes) {
    labels.push(value)
  }

  return { maxMinutes, stepMinutes, labels }
}

/** 将柱顶展示的时长向上取整为分钟。
 * @param durationSeconds - 单个趋势数据的学习秒数。
 */
export function getDisplayDurationMinutes(durationSeconds: number): number {
  return Math.ceil(Math.max(0, durationSeconds) / MINUTE_SECONDS)
}

/** 计算 Y 轴刻度在绘图区内的垂直位置。
 * @param labelMinutes - 当前刻度的分钟数。
 * @param axisMaxMinutes - Y 轴分钟上限。
 */
export function getTrendAxisLabelPosition(labelMinutes: number, axisMaxMinutes: number): string {
  if (axisMaxMinutes <= 0) return '0%'

  const percent = (labelMinutes / axisMaxMinutes) * 100
  return `${Math.max(0, Math.min(100, percent))}%`
}

/** 根据分钟刻度上限计算趋势柱高度。
 * @param durationSeconds - 单个趋势数据的学习秒数。
 * @param axisMaxMinutes - Y 轴分钟上限。
 */
export function getTrendHeight(durationSeconds: number, axisMaxMinutes: number): string {
  if (durationSeconds <= 0 || axisMaxMinutes <= 0) return '0%'

  const percent = (durationSeconds / (axisMaxMinutes * MINUTE_SECONDS)) * 100
  return `${Math.max(4, Math.min(100, percent))}%`
}
