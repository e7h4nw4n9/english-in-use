function pad2(value: number): string {
  return String(value).padStart(2, '0')
}

/**
 * 将日期格式化为本地 YYYY-MM-DD。
 * @param date - 需要格式化的日期，默认为当前时间。
 */
export function formatLocalDate(date = new Date()): string {
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`
}

function parseYmdLocal(value: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value.trim())
  if (!match) return null

  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  if (month < 1 || month > 12 || day < 1 || day > 31) return null

  const date = new Date(year, month - 1, day, 12, 0, 0, 0)
  if (date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day) {
    return null
  }

  return date
}

/**
 * 将 ISO 时间格式化为本地分钟级文本。
 * @param value - ISO 日期时间字符串。
 */
export function formatIsoToLocalMinute(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  const year = date.getFullYear()
  const month = pad2(date.getMonth() + 1)
  const day = pad2(date.getDate())
  const hour = pad2(date.getHours())
  const minute = pad2(date.getMinutes())

  return `${year}-${month}-${day} ${hour}:${minute}`
}

/** 生成开始日期到结束日期之间的全部本地日期，包含边界。 */
export function generateDateRange(start: string, end: string): string[] {
  const startDate = parseYmdLocal(start)
  const endDate = parseYmdLocal(end)
  if (!startDate || !endDate || startDate > endDate) return []

  const dates: string[] = []
  const curr = new Date(startDate)
  const last = new Date(endDate)

  // 按本地日历日期迭代，避免夏令时边界导致日期偏移。
  curr.setHours(12, 0, 0, 0)
  last.setHours(12, 0, 0, 0)

  while (curr <= last) {
    const y = curr.getFullYear()
    const m = pad2(curr.getMonth() + 1)
    const d = pad2(curr.getDate())
    dates.push(`${y}-${m}-${d}`)
    curr.setDate(curr.getDate() + 1)
  }
  return dates
}

/** 生成开始月份到结束月份之间的全部月份，包含边界。 */
export function generateMonthRange(start: string, end: string): string[] {
  const startDate = parseYmdLocal(start)
  const endDate = parseYmdLocal(end)
  if (!startDate || !endDate || startDate > endDate) return []

  const months: string[] = []
  const curr = new Date(startDate)
  const last = new Date(endDate)

  curr.setDate(1)
  curr.setHours(12, 0, 0, 0)
  last.setDate(1)
  last.setHours(12, 0, 0, 0)

  while (curr <= last) {
    const y = curr.getFullYear()
    const m = pad2(curr.getMonth() + 1)
    months.push(`${y}-${m}`)
    curr.setMonth(curr.getMonth() + 1)
  }
  return months
}
