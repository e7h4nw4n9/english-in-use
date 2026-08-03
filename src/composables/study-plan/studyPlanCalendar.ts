export type CalendarViewMode = 'week' | 'month'

/** 将本地日期格式化为 YYYY-MM-DD。
 * @param date - 待格式化的本地日期。
 */
export function formatDate(date: Date): string {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

/** 将 YYYY-MM-DD 解析为本地零点日期。
 * @param date - YYYY-MM-DD 日期文本。
 */
export function parseDate(date: string): Date {
  return new Date(`${date}T00:00:00`)
}

/** 增减指定日期的自然日。
 * @param date - 基准日期。
 * @param days - 增减的天数。
 */
export function addDays(date: Date, days: number): Date {
  const next = new Date(date)
  next.setDate(next.getDate() + days)
  return next
}

/** 增减指定日期的自然月。
 * @param date - 基准日期。
 * @param months - 增减的月数。
 */
export function addMonths(date: Date, months: number): Date {
  const next = new Date(date.getFullYear(), date.getMonth(), 1)
  next.setMonth(next.getMonth() + months)
  return next
}

/** 计算日期所在周的周一。
 * @param date - 基准日期。
 */
export function startOfWeek(date: Date): Date {
  const copy = new Date(date)
  const day = copy.getDay()
  const diff = day === 0 ? -6 : 1 - day
  copy.setDate(copy.getDate() + diff)
  copy.setHours(0, 0, 0, 0)
  return copy
}

/** 计算日期所在周的周日。
 * @param date - 基准日期。
 */
export function endOfWeek(date: Date): Date {
  return addDays(startOfWeek(date), 6)
}

/** 计算日期所在月份的首日。
 * @param date - 基准日期。
 */
export function startOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), 1)
}

/** 计算日期所在月份的末日。
 * @param date - 基准日期。
 */
export function endOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth() + 1, 0)
}

/** 根据日历视图和锚点计算查询日期范围。
 * @param mode - 周或月视图模式。
 * @param anchor - 当前日历锚点。
 */
export function getRange(mode: CalendarViewMode, anchor: Date): { start: string; end: string } {
  if (mode === 'week') {
    return { start: formatDate(startOfWeek(anchor)), end: formatDate(endOfWeek(anchor)) }
  }
  return { start: formatDate(startOfMonth(anchor)), end: formatDate(endOfMonth(anchor)) }
}
