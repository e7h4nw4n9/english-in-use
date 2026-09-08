export interface DictionaryAuthStatus {
  authenticated: boolean
  cachedPhone?: string | null
}

export interface DictionarySearchResult {
  wordId: string
  word: string
  partsOfSpeech: string[]
  definitionEng: string
  definitionZh: string
}

export interface DictionaryAudioResponse {
  bytes: number[]
  mimeType: string
}

export interface DictionaryGraphItem {
  id: number
  file: string
  fileSub: string
  english: string
  chinese: string
}

export interface DictionaryDailyTip {
  id: number
  wordId: string
  word: string
  pos: string
  unbox: string
}

export interface DictionaryImageResponse {
  bytes: number[]
  mimeType: string
}

export interface DictionaryTipFragment {
  text: string
  bold: boolean
  italic: boolean
  chinese: boolean
  breakAfter: boolean
}

export interface DictionaryTipBlock {
  kind: 'heading1' | 'heading2' | 'paragraph' | 'listItem'
  fragments: DictionaryTipFragment[]
}

export interface ParsedDictionaryTip {
  id: number
  wordId: string
  word: string
  pos: string
  title: string
  subtitle: string
  category: string
  blocks: DictionaryTipBlock[]
}

export interface WordbookEntry {
  wordId: string
  word: string
  definitionEng: string
  definitionZh: string
  createdAt?: string
}

export interface DictionaryFragment {
  text: string
  bold: boolean
  italic: boolean
}

export interface DictionaryPronunciation {
  region: string
  phonetic: string
  audio: string
}

export interface DictionaryExample {
  english: DictionaryFragment[]
  chinese: string
  audios: Array<{ region: string; name: string }>
}

export interface DictionarySense {
  id: string
  sectionTitle: string
  marker: string
  grammar: string[]
  english: string
  chinese: string
  notes: string
  examples: DictionaryExample[]
}

export type DictionarySectionKind = 'definitions' | 'idioms' | 'derivatives' | 'usage' | 'related'

export interface DictionarySectionItem {
  id: string
  heading: string
  partsOfSpeech: string[]
  pronunciations: DictionaryPronunciation[]
  senses: DictionarySense[]
  text: string
}

export interface DictionaryContentSection {
  key: DictionarySectionKind
  title: string
  items: DictionarySectionItem[]
}

export interface DictionaryPartOfSpeechGroup {
  key: string
  label: string
  badges: string[]
  inflection: DictionaryFragment[]
  pronunciations: DictionaryPronunciation[]
  sections: DictionaryContentSection[]
}

export interface ParsedDictionaryEntry {
  id: string
  word: string
  partOfSpeechGroups: DictionaryPartOfSpeechGroup[]
}
