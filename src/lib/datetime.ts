function pad2(value: number): string {
  return String(value).padStart(2, '0')
}

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

/**
 * Generates an array of dates (YYYY-MM-DD) between start and end (inclusive)
 */
export function generateDateRange(start: string, end: string): string[] {
  const dates: string[] = []
  const curr = new Date(start)
  const last = new Date(end)

  // Normalize to UTC midnight to avoid DST issues during iteration
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

/**
 * Generates an array of months (YYYY-MM) between start and end (inclusive)
 */
export function generateMonthRange(start: string, end: string): string[] {
  const months: string[] = []
  const curr = new Date(start)
  const last = new Date(end)

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
