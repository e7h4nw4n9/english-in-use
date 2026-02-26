import { describe, it, expect, vi } from 'vitest'
import { resolvePageResource, resolveBookAsset } from '../books'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path) => `asset://${path}`),
}))

describe('books api', () => {
  it('resolvePageResource should return converted src', async () => {
    vi.mocked(invoke).mockResolvedValue('/path/to/img.jpg')
    const result = await resolvePageResource('test', '12')
    expect(invoke).toHaveBeenCalledWith('resolve_page_resource', {
      productCode: 'test',
      pageLabel: '12',
    })
    expect(convertFileSrc).toHaveBeenCalledWith('/path/to/img.jpg')
    expect(result).toBe('asset:///path/to/img.jpg')
  })

  it('resolveBookAsset should return converted src', async () => {
    vi.mocked(invoke).mockResolvedValue('/path/to/audio.mp3')
    const result = await resolveBookAsset('test', 'audio.mp3')
    expect(invoke).toHaveBeenCalledWith('resolve_book_asset', {
      productCode: 'test',
      relativePath: 'audio.mp3',
    })
    expect(convertFileSrc).toHaveBeenCalledWith('/path/to/audio.mp3')
    expect(result).toBe('asset:///path/to/audio.mp3')
  })
})
