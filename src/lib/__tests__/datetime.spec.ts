import { describe, expect, it } from 'vitest'
import { formatIsoToLocalMinute } from '../datetime'

function localMinuteString(input: string): string {
  const date = new Date(input)
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  const hour = String(date.getHours()).padStart(2, '0')
  const minute = String(date.getMinutes()).padStart(2, '0')
  return `${year}-${month}-${day} ${hour}:${minute}`
}

describe('formatIsoToLocalMinute', () => {
  it('formats UTC ISO timestamp using local timezone clock time', () => {
    const input = '2026-03-01T00:01:10.000Z'
    expect(formatIsoToLocalMinute(input)).toBe(localMinuteString(input))
  })

  it('formats ISO timestamp without seconds to minute precision', () => {
    const input = '2026-03-01T09:08:00.000Z'
    const result = formatIsoToLocalMinute(input)
    expect(result).toBe(localMinuteString(input))
    expect(result).toHaveLength(16)
    expect(result.includes('T')).toBe(false)
  })

  it('falls back to original value when timestamp is invalid', () => {
    const invalid = 'invalid-timestamp'
    expect(formatIsoToLocalMinute(invalid)).toBe(invalid)
  })
})
