import type {
  DictionaryContentSection,
  DictionaryExample,
  DictionaryFragment,
  DictionaryPartOfSpeechGroup,
  DictionaryPronunciation,
  DictionarySectionItem,
  DictionarySectionKind,
  DictionarySense,
  ParsedDictionaryEntry,
} from './types'

type JsonObject = Record<string, any>

function asObject(value: unknown): JsonObject {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as JsonObject) : {}
}

function asArray(value: unknown): JsonObject[] {
  return Array.isArray(value) ? value.filter((item) => item && typeof item === 'object') : []
}

function fragments(value: unknown): DictionaryFragment[] {
  return asArray(value)
    .map((item) => ({
      text: typeof item.value === 'string' ? item.value : '',
      bold: item.bold === 1 || item.bold === '1',
      italic: item.font_Italic === 1 || item.font_Italic === '1',
    }))
    .filter((item) => item.text)
}

function plainText(value: unknown): string {
  return fragments(value)
    .map((item) => item.text)
    .join('')
    .trim()
}

function nestedText(value: unknown): string {
  if (Array.isArray(value)) return value.map(nestedText).filter(Boolean).join(' ')
  if (value && typeof value === 'object') {
    const object = value as JsonObject
    if (typeof object.value === 'string') return object.value
    return Object.values(object).map(nestedText).filter(Boolean).join(' ')
  }
  return ''
}

/** 提取补充字段文本；结构化 unbox 不能降级拼接为普通文本。 */
function supplementaryText(data: JsonObject, field: string): string {
  return field === 'unbox' ? plainText(data[field]) : nestedText(data[field])
}

function extractBadges(...values: string[]): string[] {
  const result: string[] = []
  const source = values.join(' ')
  for (const match of source.matchAll(/CEFR_([A-C][12])/gi)) result.push(match[1].toUpperCase())
  for (const match of source.matchAll(/\[(CET\d|NETM)\]/gi)) result.push(match[1].toUpperCase())
  return [...new Set(result)]
}

function parsePronunciations(value: unknown): DictionaryPronunciation[] {
  return asArray(value).map((item) => ({
    region: String(item.geo || ''),
    phonetic: String(item.phon || ''),
    audio: String(item.audio || ''),
  }))
}

function parseExample(value: unknown): DictionaryExample {
  const example = asObject(value)
  return {
    english: fragments(example.x_eng),
    chinese: plainText(example.x_simp),
    audios: asArray(example.xaudio)
      .map((item) => asObject(item.value))
      .filter((item) => typeof item.url === 'string')
      .map((item) => ({ region: item.geo === 'n_am' ? 'NAmE' : 'BrE', name: item.url })),
  }
}

interface SenseSource {
  value: JsonObject
  sectionTitle: string
}

function collectDefinitionSources(data: JsonObject): SenseSource[] {
  const result: SenseSource[] = []
  result.push(...asArray(data.sn_g).map((value) => ({ value, sectionTitle: '' })))
  for (const shortcut of asArray(data.shcut_g)) {
    const sectionTitle = cleanSenseMarker(plainText(shortcut.shcut_name), [])
    result.push(...asArray(shortcut.sn_g).map((value) => ({ value, sectionTitle })))
  }
  return result
}

function extractGrammar(value: unknown): string[] {
  return fragments(value)
    .flatMap((item) => [...item.text.matchAll(/\[([^\]]+)\]/g)])
    .map((match) => match[1].trim())
    .filter((text) => text && !/^(Ox\d+|CEFR_)/i.test(text))
}

function cleanSenseMarker(value: string, grammarItems: string[]): string {
  let result = value.replace(/\[(?:Ox\d+[^\]]*|CEFR_[^\]]*)\]/gi, '')
  for (const grammar of grammarItems) {
    result = result.replace(`[${grammar}]`, '')
  }
  return result
    .replace(/\[\s*(?:,\s*)?\]/g, '')
    .replace(/\s+,/g, ',')
    .replace(/\s{2,}/g, ' ')
    .trim()
}

function parseSense(source: SenseSource, index: number): DictionarySense {
  const value = source.value
  const marker = plainText(value.sng_text)
  const grammar = extractGrammar(value.sng_text)
  return {
    id: String(value.id || index),
    sectionTitle: source.sectionTitle,
    marker: cleanSenseMarker(marker, grammar),
    grammar,
    english: plainText(value.def_eng),
    chinese: plainText(value.def_simp),
    notes: cleanSenseMarker(
      [nestedText(value.un), plainText(value.unbox), nestedText(value.xrgs)]
        .filter(Boolean)
        .join(' ')
        .trim(),
      [],
    ),
    examples: asArray(value.x_gs).map(parseExample),
  }
}

const SECTION_TITLES: Record<DictionarySectionKind, string> = {
  definitions: '释义',
  idioms: '习语',
  derivatives: '派生词',
  usage: '用法说明',
  related: '相关词',
}

const SECTION_ORDER: DictionarySectionKind[] = [
  'definitions',
  'idioms',
  'derivatives',
  'usage',
  'related',
]

/** 创建一个词典内容段，仅保留实际包含内容的条目。 */
function createSection(
  key: DictionarySectionKind,
  items: DictionarySectionItem[],
): DictionaryContentSection | null {
  const populatedItems = items.filter(
    (item) => item.heading || item.text || item.senses.length || item.pronunciations.length,
  )
  return populatedItems.length ? { key, title: SECTION_TITLES[key], items: populatedItems } : null
}

/** 解析当前词性的普通释义和分类释义。 */
function parseDefinitions(data: JsonObject): DictionaryContentSection | null {
  const senses = collectDefinitionSources(data).map(parseSense)
  return createSection('definitions', [
    {
      id: 'definitions',
      heading: '',
      partsOfSpeech: [],
      pronunciations: [],
      senses,
      text: '',
    },
  ])
}

/** 将当前词性的习语转换为带标题的独立条目。 */
function parseIdioms(data: JsonObject): DictionaryContentSection | null {
  const items: DictionarySectionItem[] = []
  for (const idiomGroup of asArray(data.idm_gs)) {
    for (const idiom of asArray(idiomGroup.idm_g)) {
      items.push({
        id: String(idiom.id || `idiom-${items.length}`),
        heading: cleanSenseMarker(plainText(idiom.idm_name), []),
        partsOfSpeech: [],
        pronunciations: [],
        senses: asArray(idiom.sn_g).map((value, index) =>
          parseSense({ value, sectionTitle: '' }, index),
        ),
        text: cleanSenseMarker(
          [nestedText(idiom.idm_text), nestedText(idiom.un), nestedText(idiom.xrgs)]
            .filter(Boolean)
            .join(' ')
            .trim(),
          [],
        ),
      })
    }
  }
  return createSection('idioms', items)
}

/** 将当前词性的派生词转换为包含自身词性、发音和释义的独立条目。 */
function parseDerivatives(data: JsonObject): DictionaryContentSection | null {
  const items = asArray(data.dr_gs).map((derivative, derivativeIndex) => {
    const top = asObject(derivative.top_g)
    const senseData = asObject(derivative.sn_gs)
    const directPartsOfSpeech = fragments(top.pos)
    const partsOfSpeech = (
      directPartsOfSpeech.length ? directPartsOfSpeech : fragments(senseData.pos)
    )
      .map((item) => item.text.trim())
      .filter(Boolean)
    return {
      id: String(derivative.id || `derivative-${derivativeIndex}`),
      heading: cleanSenseMarker(plainText(top.h), []),
      partsOfSpeech,
      pronunciations: parsePronunciations(top.prongs),
      senses: asArray(senseData.sn_g).map((value, index) =>
        parseSense({ value, sectionTitle: '' }, index),
      ),
      text: cleanSenseMarker(
        [nestedText(senseData.un), plainText(senseData.unbox), nestedText(senseData.xrgs)]
          .filter(Boolean)
          .join(' ')
          .trim(),
        [],
      ),
    }
  })
  return createSection('derivatives', items)
}

/** 从指定字段收集用法或相关词文本。 */
function parseTextSection(
  data: JsonObject,
  key: Extract<DictionarySectionKind, 'usage' | 'related'>,
  fields: string[],
): DictionaryContentSection | null {
  const text = cleanSenseMarker(
    fields
      .map((field) => supplementaryText(data, field))
      .filter(Boolean)
      .join(' ')
      .trim(),
    [],
  )
  return createSection(key, [
    {
      id: key,
      heading: '',
      partsOfSpeech: [],
      pronunciations: [],
      senses: [],
      text,
    },
  ])
}

/** 按固定展示顺序解析一个数据节点中的全部词典内容段。 */
function parseSections(data: JsonObject): DictionaryContentSection[] {
  return [
    parseDefinitions(data),
    parseIdioms(data),
    parseDerivatives(data),
    parseTextSection(data, 'usage', ['un', 'unbox']),
    parseTextSection(data, 'related', ['xrgs', 'xrgs_subtren']),
  ].filter((section): section is DictionaryContentSection => Boolean(section))
}

/** 合并词性内容与词条级补充内容，并保持目录顺序稳定。 */
function mergeSections(
  groupSections: DictionaryContentSection[],
  sharedSections: DictionaryContentSection[],
): DictionaryContentSection[] {
  return SECTION_ORDER.flatMap((key) => {
    const items = [...groupSections, ...sharedSections]
      .filter((section) => section.key === key)
      .flatMap((section, sectionIndex) =>
        section.items.map((item, itemIndex) => ({
          ...item,
          id: `${sectionIndex}-${itemIndex}-${item.id}`,
        })),
      )
    const section = createSection(key, items)
    return section ? [section] : []
  })
}

function mergeBadges(globalBadges: string[], groupSources: string[]): string[] {
  const groupBadges = extractBadges(...groupSources)
  const groupCefr = groupBadges.filter((badge) => /^[A-C][12]$/.test(badge))
  const globalCefr = globalBadges.filter((badge) => /^[A-C][12]$/.test(badge))
  const commonBadges = globalBadges.filter((badge) => !/^[A-C][12]$/.test(badge))
  return [...new Set([...(groupCefr.length ? groupCefr : globalCefr), ...commonBadges])]
}

/** 将一个词性分组转换为稳定的展示模型。 */
function parsePartOfSpeechGroup(
  group: JsonObject,
  index: number,
  top: JsonObject,
  globalBadges: string[],
  fallbackPartsOfSpeech: string[],
  sharedSections: DictionaryContentSection[],
): DictionaryPartOfSpeechGroup {
  const data = asObject(group.sngs_data)
  const groupTop = asObject(group.top_data)
  const definitionSources = collectDefinitionSources(data)
  const directPartsOfSpeech = fragments(data.pos)
  const groupPartsOfSpeech = directPartsOfSpeech.length
    ? directPartsOfSpeech
    : fragments(groupTop.pos)
  const normalizedPartsOfSpeech = groupPartsOfSpeech.map((item) => item.text.trim()).filter(Boolean)
  const label = normalizedPartsOfSpeech.join(' / ') || fallbackPartsOfSpeech.join(' / ') || '释义'
  const groupInflection = fragments(groupTop.top_text)
  const groupPronunciations = parsePronunciations(groupTop.prongs)
  return {
    key: `${index}-${label}`,
    label,
    badges: mergeBadges(globalBadges, [
      nestedText(data.subentry_cefr),
      ...definitionSources.map((source) => plainText(source.value.sng_text)),
    ]),
    inflection: groupInflection.some((item) => item.text.trim())
      ? groupInflection
      : fragments(top.top_text),
    pronunciations: groupPronunciations.length
      ? groupPronunciations
      : parsePronunciations(top.prongs),
    sections: mergeSections(parseSections(data), sharedSections),
  }
}

/** 将牛津接口的动态 JSON 转换为稳定的展示模型。 */
export function parseDictionaryEntry(raw: Record<string, unknown>): ParsedDictionaryEntry {
  const data = asObject(raw)
  const wordBody = asObject(data.word_body)
  const top = asObject(wordBody.top_data)
  const headings = fragments(top.h)
  const word = headings[0]?.text || String(data.word || '')
  const badgeSource = headings.slice(1).map((item) => item.text)
  const globalBadges = extractBadges(...badgeSource)
  const fallbackPartsOfSpeech = fragments(top.pos)
    .map((item) => item.text.trim())
    .filter(Boolean)
  const rawGroups = asArray(wordBody.sngs_data)
  const groups = rawGroups.length ? rawGroups : [{ sngs_data: wordBody }]
  const sharedSections = [
    parseTextSection(wordBody, 'usage', ['un', 'unbox']),
    parseTextSection(wordBody, 'related', ['xrgs', 'xrefs']),
  ].filter((section): section is DictionaryContentSection => Boolean(section))
  return {
    id: String(data.id || headings[0]?.text || ''),
    word,
    partOfSpeechGroups: groups.map((group, index) =>
      parsePartOfSpeechGroup(
        group,
        index,
        top,
        globalBadges,
        fallbackPartsOfSpeech,
        sharedSections,
      ),
    ),
  }
}
