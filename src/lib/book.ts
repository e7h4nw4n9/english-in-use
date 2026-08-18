import type { Book } from '@/types'

/** 返回书籍用于界面展示的名称。
 * @param book - 包含完整名称和可选简称的书籍。
 */
export function getBookDisplayTitle(book: Pick<Book, 'short_title' | 'title'>): string {
  return book.short_title?.trim() || book.title
}
