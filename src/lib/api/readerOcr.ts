import { invoke, isTauri } from '@tauri-apps/api/core'

export interface WordRect {
  x: number
  y: number
  width: number
  height: number
}
export interface RecognizedWord {
  text: string
  rect: WordRect
}
export interface RecognizedPage {
  width: number
  height: number
  lines: Array<{ text: string; words: RecognizedWord[] }>
}

/** 识别指定书页；浏览器预览不使用替代 OCR 服务。 */
export function recognizeReaderPage(
  productCode: string,
  pageLabel: string,
): Promise<RecognizedPage> {
  if (!isTauri()) return Promise.reject(new Error('请在 macOS 或 iPad 应用中使用原生选词'))
  return invoke('recognize_reader_page', { productCode, pageLabel })
}

/** 通过原生剪贴板复制一个单词，不读取剪贴板。 */
export function copyReaderWord(word: string): Promise<void> {
  return invoke('copy_reader_word', { word })
}

/** 判断归一化坐标是否命中矩形。 */
export function containsPoint(rect: WordRect, x: number, y: number): boolean {
  return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
}

/** 排除与热点相交的文字，不让隐藏热点意外变为可选词区域。 */
export function intersectsRect(a: WordRect, b: WordRect): boolean {
  return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

/** 只接受实际命中的单词，空白处不猜测最近的文字。 */
export function findReaderWord(
  page: RecognizedPage,
  x: number,
  y: number,
  hotspots: WordRect[],
): RecognizedWord | undefined {
  if (hotspots.some((rect) => containsPoint(rect, x, y))) return
  return page.lines
    .flatMap((line) => line.words)
    .find(
      (word) =>
        containsPoint(word.rect, x, y) && !hotspots.some((rect) => intersectsRect(word.rect, rect)),
    )
}
