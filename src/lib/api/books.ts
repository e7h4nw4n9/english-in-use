import { invoke } from '@tauri-apps/api/core'
import { Book, BookGroup } from '../../types'

export async function getBooks(group?: BookGroup): Promise<Book[]> {
  return await invoke('get_books', { group })
}

export async function getBookCover(book: Book): Promise<Uint8Array> {
  const bytes = await invoke<number[] | Uint8Array>('get_book_cover', { book })
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
}

export function bytesToImageUrl(bytes: Uint8Array, mimeType: string = 'image/jpeg'): string {
  if (!bytes || bytes.length === 0) return ''
  const blob = new Blob([bytes], { type: mimeType })
  return URL.createObjectURL(blob)
}
