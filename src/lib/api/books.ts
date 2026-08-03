import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { Book, BookGroup, BookMetadata, ReadingProgress } from '../../types'

/**
 * 获取可选分组下的图书列表。
 * @param group - 可选的图书分组。
 */
export async function getBooks(group?: BookGroup): Promise<Book[]> {
  return await invoke('get_books', { group })
}

/**
 * 获取图书封面字节。
 * @param book - 目标图书。
 */
export async function getBookCover(book: Book): Promise<Uint8Array> {
  const bytes = await invoke<number[] | Uint8Array>('get_book_cover', { book })
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
}

/**
 * 获取阅读器所需的图书聚合元数据。
 * @param productCode - 图书产品码。
 */
export async function getBookMetadata(productCode: string): Promise<BookMetadata> {
  return await invoke('get_book_metadata', { productCode })
}

/**
 * 解析页面图片资源 URL。
 * @param productCode - 图书产品码。
 * @param pageLabel - 页面标签。
 */
export async function resolvePageResource(productCode: string, pageLabel: string): Promise<string> {
  const path = await invoke<string>('resolve_page_resource', { productCode, pageLabel })
  return convertFileSrc(path)
}

/**
 * 解析图书相对资产 URL。
 * @param productCode - 图书产品码。
 * @param relativePath - 图书目录内相对路径。
 */
export async function resolveBookAsset(productCode: string, relativePath: string): Promise<string> {
  const path = await invoke<string>('resolve_book_asset', { productCode, relativePath })
  return convertFileSrc(path)
}

/**
 * 解析练习入口资源 URL。
 * @param productCode - 图书产品码。
 * @param resourceId - 练习资源标识。
 */
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

/**
 * 获取已经过资源改写的练习 HTML。
 * @param productCode - 图书产品码。
 * @param resourceId - 练习资源标识。
 */
export async function getExerciseHtml(
  productCode: string,
  resourceId: string,
): Promise<ExerciseHtmlResponse> {
  return await invoke<ExerciseHtmlResponse>('get_exercise_html', { productCode, resourceId })
}

/**
 * 获取图书阅读进度。
 * @param productCode - 图书产品码。
 */
export async function getReadingProgress(productCode: string): Promise<ReadingProgress | null> {
  return await invoke('get_reading_progress', { productCode })
}

/**
 * 保存图书阅读位置和视图状态。
 * @param productCode - 图书产品码。
 * @param resourceId - 当前页面资源标识。
 * @param pageLabel - 当前页面标签。
 * @param scale - 缩放比例。
 * @param offsetX - 水平偏移量。
 * @param offsetY - 垂直偏移量。
 */
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

/**
 * 将图片字节转换为浏览器可展示的 Data URL。
 * @param bytes - 图片字节。
 * @param mimeType - 图片 MIME 类型。
 */
export function bytesToImageUrl(bytes: Uint8Array, mimeType: string = 'image/jpeg'): string {
  if (!bytes || bytes.length === 0) return ''
  const blob = new Blob([bytes], { type: mimeType })
  return URL.createObjectURL(blob)
}
