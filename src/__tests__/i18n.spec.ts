import { describe, it, expect } from 'vitest'
import en from '../locales/en.json'
import zh from '../locales/zh.json'

function getDeepKeys(obj: any, prefix = ''): string[] {
  return Object.keys(obj).reduce((res: string[], el) => {
    if (Array.isArray(obj[el])) {
      return [...res, prefix + el]
    } else if (typeof obj[el] === 'object' && obj[el] !== null) {
      return [...res, ...getDeepKeys(obj[el], prefix + el + '.')]
    }
    return [...res, prefix + el]
  }, [])
}

describe('i18n keys consistency', () => {
  it('should have the same keys for en and zh', () => {
    const enKeys = getDeepKeys(en).sort()
    const zhKeys = getDeepKeys(zh).sort()

    // Find missing keys in zh
    const missingInZh = enKeys.filter((key) => !zhKeys.includes(key))
    const extraInZh = zhKeys.filter((key) => !enKeys.includes(key))

    if (missingInZh.length > 0 || extraInZh.length > 0) {
      console.error('I18n keys mismatch:')
      if (missingInZh.length > 0) console.error('Missing in zh.json:', missingInZh)
      if (extraInZh.length > 0) console.error('Extra in zh.json:', extraInZh)
    }

    expect(missingInZh).toEqual([])
    expect(extraInZh).toEqual([])
  })
})
