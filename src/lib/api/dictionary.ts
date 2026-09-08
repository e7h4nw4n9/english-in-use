import { invoke } from '@tauri-apps/api/core'
import type {
  DictionaryAudioResponse,
  DictionaryAuthStatus,
  DictionaryDailyTip,
  DictionaryGraphItem,
  DictionaryImageResponse,
  DictionarySearchResult,
  WordbookEntry,
} from '@/features/dictionary/types'

/** 读取词典登录状态。 */
export const getDictionaryAuthStatus = () => invoke<DictionaryAuthStatus>('dictionary_auth_status')

/** 发送词典登录验证码。 */
export const sendDictionaryVerifyCode = (phone: string) =>
  invoke<void>('dictionary_send_verify_code', { phone })

/** 使用短信验证码登录词典。 */
export const loginDictionary = (phone: string, code: string) =>
  invoke<void>('dictionary_login', { phone, code })

/** 退出词典账号。 */
export const logoutDictionary = () => invoke<void>('dictionary_logout')

/** 随机获取一条图解词汇。 */
export const getRandomDictionaryGraph = () => invoke<DictionaryGraphItem>('dictionary_random_graph')

/** 获取每日实用贴士。 */
export const getDictionaryDailyTip = () => invoke<DictionaryDailyTip>('dictionary_daily_tip')

/** 下载图解词汇缩略图或完整图片。 */
export const getDictionaryGraphImage = (path: string) =>
  invoke<DictionaryImageResponse>('dictionary_graph_image', { path })

/** 查询词典候选项。 */
export const searchDictionary = (word: string) =>
  invoke<DictionarySearchResult[]>('dictionary_search', { word })

/** 获取词条详情。 */
export const getDictionaryWordDetail = (wordId: string) =>
  invoke<Record<string, unknown>>('dictionary_word_detail', { wordId })

/** 获取词条或例句音频字节。 */
export const getDictionaryAudio = (kind: 'word' | 'example', name: string) =>
  invoke<DictionaryAudioResponse>('dictionary_audio', { kind, name })

/** 读取单词本。 */
export const listWordbookEntries = () => invoke<WordbookEntry[]>('list_wordbook_entries')

/** 新增或更新单词本词条。 */
export const addWordbookEntry = (entry: WordbookEntry) =>
  invoke<void>('add_wordbook_entry', { entry })

/** 删除单词本词条。 */
export const removeWordbookEntry = (wordId: string) =>
  invoke<void>('remove_wordbook_entry', { wordId })
