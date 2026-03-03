import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { Book, BookGroup, BookMetadata, ReadingProgress } from '../../types'

export async function getBooks(group?: BookGroup): Promise<Book[]> {
  return await invoke('get_books', { group })
}

export async function getBookCover(book: Book): Promise<Uint8Array> {
  const bytes = await invoke<number[] | Uint8Array>('get_book_cover', { book })
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
}

export async function getBookMetadata(productCode: string): Promise<BookMetadata> {
  return await invoke('get_book_metadata', { productCode })
}

export async function resolvePageResource(productCode: string, pageLabel: string): Promise<string> {
  const path = await invoke<string>('resolve_page_resource', { productCode, pageLabel })
  return convertFileSrc(path)
}

export async function resolveBookAsset(productCode: string, relativePath: string): Promise<string> {
  const path = await invoke<string>('resolve_book_asset', { productCode, relativePath })
  return convertFileSrc(path)
}

export async function resolveExerciseResource(
  productCode: string,
  resourceId: string,
): Promise<string> {
  const path = await invoke<string>('resolve_exercise_resource', { productCode, resourceId })
  return convertFileSrc(path)
}

export interface ExerciseHtmlResponse {
  html: string
  url: string
}

export async function getExerciseHtml(
  productCode: string,
  resourceId: string,
): Promise<ExerciseHtmlResponse> {
  return await invoke<ExerciseHtmlResponse>('get_exercise_html', { productCode, resourceId })
}

export async function getReadingProgress(productCode: string): Promise<ReadingProgress | null> {
  return await invoke('get_reading_progress', { productCode })
}

export async function updateReadingProgress(
  productCode: string,
  resourceId: string | null,
  pageLabel: string | null,
  scale: number,
  offsetX: number,
  offsetY: number,
): Promise<void> {
  return await invoke('update_reading_progress', {
    productCode,
    resourceId,
    pageLabel,
    scale,
    offsetX,
    offsetY,
  })
}

export function bytesToImageUrl(bytes: Uint8Array, mimeType: string = 'image/jpeg'): string {
  if (!bytes || bytes.length === 0) return ''
  const blob = new Blob([bytes], { type: mimeType })
  return URL.createObjectURL(blob)
}
