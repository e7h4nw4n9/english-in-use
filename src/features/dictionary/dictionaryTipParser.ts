import type {
  DictionaryDailyTip,
  DictionaryTipBlock,
  DictionaryTipFragment,
  ParsedDictionaryTip,
} from './types'

type JsonObject = Record<string, unknown>

function asObject(value: unknown): JsonObject {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as JsonObject) : {}
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function parseFragments(value: unknown): DictionaryTipFragment[] {
  return asArray(value)
    .map((item) => asObject(item))
    .map((item) => {
      const tag = typeof item.tag === 'string' ? item.tag : ''
      return {
        text: typeof item.value === 'string' ? item.value : '',
        bold: Number(item.bold) === 1 || tag === 'eb',
        italic: String(item.font_Italic) === '1',
        chinese: tag === 'simp' || tag === 'trad',
        breakAfter: Number(item.if_newline) === 1,
      }
    })
    .filter((fragment) => fragment.text || fragment.breakAfter)
}

function appendBlock(
  blocks: DictionaryTipBlock[],
  kind: DictionaryTipBlock['kind'],
  value: unknown,
) {
  const fragments = parseFragments(value)
  if (fragments.length) blocks.push({ kind, fragments })
}

/** 将实用贴士的动态 unbox JSON 转换为安全的展示模型。 */
export function parseDictionaryTip(tip: DictionaryDailyTip): ParsedDictionaryTip {
  let unbox: JsonObject
  try {
    unbox = asObject(JSON.parse(tip.unbox))
  } catch {
    throw new Error('实用贴士内容格式错误')
  }

  const tile = asObject(unbox.tile)
  const title = typeof tile.type === 'string' ? tile.type.trim() : ''
  const subtitle = typeof tile.eng === 'string' ? tile.eng.trim() : ''
  const category = title.match(/[\u3400-\u9fff].*$/)?.[0]?.trim() || title
  const blocks: DictionaryTipBlock[] = []

  for (const rawBlock of asArray(unbox.body)) {
    const block = asObject(rawBlock)
    appendBlock(blocks, 'heading1', block.h1)
    appendBlock(blocks, 'heading2', block.h2)
    appendBlock(blocks, 'paragraph', block.p)
    for (const rawListItem of asArray(block.ul)) {
      appendBlock(blocks, 'listItem', asObject(rawListItem).li)
    }
  }

  if (!title || !subtitle || !blocks.length) {
    throw new Error('实用贴士内容不完整')
  }

  return {
    id: tip.id,
    wordId: tip.wordId,
    word: tip.word,
    pos: tip.pos,
    title,
    subtitle,
    category,
    blocks,
  }
}
