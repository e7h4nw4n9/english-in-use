<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { theme } from 'ant-design-vue'
import { ArrowLeftOutlined, SearchOutlined } from '@ant-design/icons-vue'
import { getDictionaryWordDetail, searchDictionary } from '@/lib/api'
import { parseDictionaryEntry } from '@/features/dictionary/dictionaryParser'
import { getReadableCommandError, parseCommandError } from '@/lib/error'
import type { DictionarySearchResult, ParsedDictionaryEntry } from '@/features/dictionary/types'
import DictionaryEntryView from './DictionaryEntryView.vue'
import DictionaryLoginModal from './DictionaryLoginModal.vue'

const props = withDefaults(defineProps<{ initialQuery?: string; compact?: boolean }>(), {
  initialQuery: '',
  compact: false,
})
const emit = defineEmits<{
  (event: 'before-audio'): void
  (event: 'detail-mode-change', visible: boolean): void
}>()

const SEARCH_DEBOUNCE_MS = 300
const query = ref(props.initialQuery)
const searching = ref(false)
const loadingDetail = ref(false)
const results = ref<DictionarySearchResult[]>([])
const entry = ref<ParsedDictionaryEntry | null>(null)
const errorMessage = ref('')
const lastCompletedQuery = ref('')
const dropdownOpen = ref(false)
const activeResultIndex = ref(-1)
const inputRef = ref<unknown>(null)
const loginOpen = ref(false)
const isComposing = ref(false)
const viewMode = ref<'search' | 'detail'>('search')
const { token } = theme.useToken()
const dropdownStyle = computed(() => ({ backgroundColor: token.value.colorBgElevated }))

let searchTimer: number | null = null
let searchRequestVersion = 0
let detailRequestVersion = 0
let activeRequestQuery = ''
let suppressNextAutomaticSearch = false
let ignoreNextSearchEvent = false
let pendingRetry: (() => Promise<void>) | null = null

const hasEmptyResult = computed(
  () =>
    !searching.value &&
    !entry.value &&
    Boolean(lastCompletedQuery.value) &&
    lastCompletedQuery.value === query.value.trim() &&
    !results.value.length &&
    !errorMessage.value,
)

const activeResultId = computed(() =>
  activeResultIndex.value >= 0 ? `dictionary-result-${activeResultIndex.value}` : undefined,
)

function clearSearchTimer() {
  if (searchTimer !== null) {
    window.clearTimeout(searchTimer)
    searchTimer = null
  }
}

function resetResults() {
  results.value = []
  dropdownOpen.value = false
  activeResultIndex.value = -1
  lastCompletedQuery.value = ''
}

/** 将鉴权失败转为登录流程，其余错误直接展示。
 * @param error - 词典命令返回的错误。
 * @param retry - 登录成功后允许执行一次的原操作。
 * @param allowLogin - 是否允许当前失败再次触发登录窗口。
 */
function handleRequestError(error: unknown, retry: () => Promise<void>, allowLogin: boolean) {
  const parsed = parseCommandError(error)
  if (allowLogin && parsed.code === 'ERR_DICTIONARY_AUTH_REQUIRED') {
    pendingRetry = retry
    loginOpen.value = true
    errorMessage.value = ''
    return
  }
  errorMessage.value = getReadableCommandError(error)
}

/** 查询词典并仅接收最新请求的结果。
 * @param requestedQuery - 本次要查询的文本。
 * @param allowLogin - 鉴权失败时是否允许打开登录窗口。
 */
async function runSearch(requestedQuery: string, allowLogin = true) {
  const normalized = requestedQuery.trim()
  if (!normalized || (searching.value && activeRequestQuery === normalized)) return

  const requestVersion = ++searchRequestVersion
  viewMode.value = 'search'
  activeRequestQuery = normalized
  searching.value = true
  loadingDetail.value = false
  entry.value = null
  results.value = []
  dropdownOpen.value = false
  activeResultIndex.value = -1
  errorMessage.value = ''
  lastCompletedQuery.value = ''

  try {
    const nextResults = await searchDictionary(normalized)
    if (requestVersion !== searchRequestVersion || query.value.trim() !== normalized) return
    results.value = nextResults
    lastCompletedQuery.value = normalized
    dropdownOpen.value = nextResults.length > 0
    activeResultIndex.value = nextResults.length > 0 ? 0 : -1
  } catch (error) {
    if (requestVersion !== searchRequestVersion || query.value.trim() !== normalized) return
    handleRequestError(error, () => runSearch(normalized, false), allowLogin)
  } finally {
    if (requestVersion === searchRequestVersion) {
      searching.value = false
      activeRequestQuery = ''
    }
  }
}

/** 取消防抖并立即查询当前输入。 */
function searchImmediately() {
  if (ignoreNextSearchEvent) {
    ignoreNextSearchEvent = false
    return
  }
  clearSearchTimer()
  void runSearch(query.value)
}

/** 在用户停止输入后执行查询。 */
function scheduleSearch() {
  clearSearchTimer()
  if (isComposing.value) return
  const normalized = query.value.trim()
  if (!normalized) return
  searchTimer = window.setTimeout(() => {
    searchTimer = null
    void runSearch(normalized)
  }, SEARCH_DEBOUNCE_MS)
}

/** 根据候选词标识获取并展示完整详情。
 * @param result - 用户选择的候选词。
 * @param allowLogin - 鉴权失败时是否允许打开登录窗口。
 */
async function openResult(result: DictionarySearchResult, allowLogin = true) {
  clearSearchTimer()
  viewMode.value = 'detail'
  dropdownOpen.value = false
  loadingDetail.value = true
  entry.value = null
  errorMessage.value = ''
  const requestVersion = ++detailRequestVersion

  try {
    const detail = await getDictionaryWordDetail(result.wordId)
    if (requestVersion !== detailRequestVersion) return
    entry.value = parseDictionaryEntry(detail)
  } catch (error) {
    if (requestVersion !== detailRequestVersion) return
    handleRequestError(error, () => openResult(result, false), allowLogin)
  } finally {
    if (requestVersion === detailRequestVersion) loadingDetail.value = false
  }
}

/** 返回查询视图并恢复进入详情前的候选列表。 */
async function backToSearch() {
  detailRequestVersion += 1
  loadingDetail.value = false
  entry.value = null
  errorMessage.value = ''
  viewMode.value = 'search'
  dropdownOpen.value = results.value.length > 0
  await nextTick()
  const candidate = inputRef.value as { focus?: () => void } | null
  candidate?.focus?.()
}

/** 处理查询框键盘导航。
 * @param event - 输入框键盘事件。
 */
function handleKeydown(event: KeyboardEvent) {
  if (event.isComposing) return
  if (event.key === 'Escape' && dropdownOpen.value) {
    event.preventDefault()
    dropdownOpen.value = false
    return
  }
  if (!dropdownOpen.value || !results.value.length) {
    if (event.key === 'Enter') searchImmediately()
    return
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    activeResultIndex.value = (activeResultIndex.value + 1) % results.value.length
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    activeResultIndex.value =
      (activeResultIndex.value - 1 + results.value.length) % results.value.length
  } else if (event.key === 'Enter' && activeResultIndex.value >= 0) {
    event.preventDefault()
    event.stopPropagation()
    ignoreNextSearchEvent = true
    window.setTimeout(() => {
      ignoreNextSearchEvent = false
    }, 0)
    void openResult(results.value[activeResultIndex.value])
  }
}

function handleCompositionEnd() {
  isComposing.value = false
  scheduleSearch()
}

/** 登录完成后仅重试一次之前失败的操作。 */
async function handleLoginSuccess() {
  loginOpen.value = false
  const retry = pendingRetry
  pendingRetry = null
  if (retry) await retry()
}

watch(query, () => {
  if (suppressNextAutomaticSearch) {
    suppressNextAutomaticSearch = false
    return
  }
  searchRequestVersion += 1
  detailRequestVersion += 1
  searching.value = false
  loadingDetail.value = false
  viewMode.value = 'search'
  entry.value = null
  errorMessage.value = ''
  resetResults()
  scheduleSearch()
})

watch(viewMode, (mode) => emit('detail-mode-change', mode === 'detail'), { immediate: true })

watch(
  () => props.initialQuery,
  async (value) => {
    clearSearchTimer()
    viewMode.value = 'search'
    entry.value = null
    if (query.value !== value) {
      suppressNextAutomaticSearch = true
      query.value = value
      await nextTick()
    }
    if (value.trim()) {
      await runSearch(value)
    } else {
      await nextTick(() => {
        const candidate = inputRef.value as { focus?: () => void } | null
        candidate?.focus?.()
      })
    }
  },
  { immediate: true },
)

onUnmounted(() => {
  clearSearchTimer()
  searchRequestVersion += 1
  detailRequestVersion += 1
})
</script>

<template>
  <section class="dictionary-lookup" :class="{ compact }">
    <div v-if="viewMode === 'search'" class="search-view">
      <div class="search-control">
        <a-input-search
          ref="inputRef"
          v-model:value="query"
          size="large"
          allow-clear
          :loading="searching"
          placeholder="输入单词或短语"
          enter-button="查询"
          autocomplete="off"
          autocapitalize="none"
          autocorrect="off"
          :spellcheck="false"
          role="combobox"
          aria-autocomplete="list"
          :aria-expanded="dropdownOpen"
          aria-controls="dictionary-search-results"
          :aria-activedescendant="activeResultId"
          @search="searchImmediately"
          @keydown="handleKeydown"
          @compositionstart="isComposing = true"
          @compositionend="handleCompositionEnd"
        >
          <template #prefix><SearchOutlined /></template>
        </a-input-search>

        <div
          v-if="dropdownOpen && results.length"
          id="dictionary-search-results"
          class="search-results"
          :style="dropdownStyle"
          role="listbox"
          aria-label="词典查询候选词"
        >
          <button
            v-for="(result, index) in results"
            :id="`dictionary-result-${index}`"
            :key="result.wordId"
            type="button"
            class="result-option"
            :class="{ active: index === activeResultIndex }"
            role="option"
            :aria-selected="index === activeResultIndex"
            @mouseenter="activeResultIndex = index"
            @click="openResult(result)"
          >
            <span class="result-heading">
              <span class="result-word">{{ result.word }}</span>
              <span v-if="result.partsOfSpeech.length" class="result-pos">
                {{ result.partsOfSpeech.join(' / ') }}
              </span>
            </span>
            <span v-if="result.definitionEng" class="result-definition">
              {{ result.definitionEng }}
            </span>
            <span v-if="result.definitionZh" class="result-definition chinese">
              {{ result.definitionZh }}
            </span>
          </button>
        </div>
      </div>

      <a-alert
        v-if="errorMessage"
        class="lookup-message"
        type="error"
        show-icon
        :message="errorMessage"
      />
      <a-empty v-else-if="hasEmptyResult" description="未找到匹配结果" />
    </div>

    <div v-else class="detail-view">
      <a-button class="operation-button detail-back" type="text" @click="backToSearch">
        <template #icon><ArrowLeftOutlined /></template>
        返回查询
      </a-button>
      <a-alert
        v-if="errorMessage"
        class="lookup-message"
        type="error"
        show-icon
        :message="errorMessage"
      />
      <div v-if="loadingDetail" class="detail-loading">
        <a-spin size="large" tip="正在加载词条…" />
      </div>
      <DictionaryEntryView v-else-if="entry" :entry="entry" @before-audio="emit('before-audio')" />
    </div>

    <DictionaryLoginModal
      :open="loginOpen"
      @cancel="loginOpen = false"
      @success="handleLoginSuccess"
    />
  </section>
</template>

<style scoped>
.dictionary-lookup {
  width: min(100%, 1180px);
  margin: 0 auto;
  padding: 24px;
}
.dictionary-lookup.compact {
  padding: 16px;
}
.search-view {
  width: min(100%, 980px);
  margin: 0 auto;
}
.search-control {
  position: relative;
  z-index: 2;
}
.lookup-message {
  margin-top: 16px;
}
.search-results {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  left: 0;
  z-index: 1000;
  max-height: min(420px, 55vh);
  overflow-y: auto;
  border: 1px solid var(--ant-color-border-secondary);
  border-radius: 10px;
  padding: 4px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.14);
}
.result-option {
  display: grid;
  width: 100%;
  gap: 3px;
  border: 0;
  border-radius: 7px;
  padding: 10px 12px;
  text-align: left;
  color: var(--ant-color-text);
  background: transparent;
  cursor: pointer;
  transition: background-color 160ms ease;
}
.result-option:hover,
.result-option.active {
  background: var(--ant-color-fill-tertiary);
}
.result-option:focus-visible {
  outline: 2px solid var(--ant-color-primary);
  outline-offset: -2px;
}
.result-heading {
  display: flex;
  align-items: baseline;
  gap: 10px;
}
.result-word {
  font-size: 17px;
  font-weight: 700;
}
.result-pos {
  color: var(--ant-color-primary);
  font-size: 13px;
}
.result-definition {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}
.result-definition.chinese {
  color: var(--ant-color-text-secondary);
}
.detail-loading {
  display: grid;
  min-height: 260px;
  place-items: center;
}
.detail-view {
  min-width: 0;
}
.detail-back {
  margin: 0 0 14px -10px;
}
.detail-back:focus-visible {
  outline: 2px solid var(--ant-color-primary);
  outline-offset: 2px;
}
:deep(.ant-empty) {
  margin-top: 64px;
}
@media (max-width: 640px) {
  .dictionary-lookup {
    padding: 14px;
  }
  .search-results {
    max-height: 48vh;
  }
}
@media (prefers-reduced-motion: reduce) {
  .result-option {
    transition: none;
  }
}
</style>
