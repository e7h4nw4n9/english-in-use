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
