<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { getDictionaryDailyTip, getDictionaryGraphImage, getRandomDictionaryGraph } from '@/lib/api'
import { getReadableCommandError, parseCommandError } from '@/lib/error'
import { parseDictionaryTip } from '@/features/dictionary/dictionaryTipParser'
import type { DictionaryGraphItem, ParsedDictionaryTip } from '@/features/dictionary/types'
import DictionaryGraphPreview from './DictionaryGraphPreview.vue'
import DictionaryLoginModal from './DictionaryLoginModal.vue'

const emit = defineEmits<{ (event: 'open-tip', tip: ParsedDictionaryTip): void }>()

const graph = ref<DictionaryGraphItem | null>(null)
const tip = ref<ParsedDictionaryTip | null>(null)
const graphLoading = ref(false)
const tipLoading = ref(false)
const graphError = ref('')
const tipError = ref('')
const thumbnailUrl = ref('')
const previewOpen = ref(false)
const previewLoading = ref(false)
const previewError = ref('')
const previewImageUrl = ref('')
const loginOpen = ref(false)
let previewRequestVersion = 0

function revokeObjectUrl(url: string) {
  if (url) URL.revokeObjectURL(url)
}

function createImageUrl(bytes: number[], mimeType: string) {
  return URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: mimeType }))
}

/** 清理当前图解词汇对应的原图预览。 */
function resetGraphPreview() {
  previewRequestVersion += 1
  previewOpen.value = false
  previewLoading.value = false
  previewError.value = ''
  revokeObjectUrl(previewImageUrl.value)
  previewImageUrl.value = ''
}

/** 处理精选内容错误，认证失败时合并为一个登录窗口。 */
function handleContentError(error: unknown, setError: (message: string) => void) {
  if (parseCommandError(error).code === 'ERR_DICTIONARY_AUTH_REQUIRED') {
    setError('登录后可查看该内容')
    loginOpen.value = true
    return
  }
  setError(getReadableCommandError(error))
}

/** 加载图解词汇及其缩略图。 */
async function loadGraph() {
  graphLoading.value = true
  graphError.value = ''
  try {
    const nextGraph = await getRandomDictionaryGraph()
    const image = await getDictionaryGraphImage(nextGraph.fileSub)
    if (graph.value?.file !== nextGraph.file) resetGraphPreview()
    revokeObjectUrl(thumbnailUrl.value)
    thumbnailUrl.value = createImageUrl(image.bytes, image.mimeType)
    graph.value = nextGraph
  } catch (error) {
    graph.value = null
    resetGraphPreview()
    revokeObjectUrl(thumbnailUrl.value)
    thumbnailUrl.value = ''
    handleContentError(error, (message) => (graphError.value = message))
  } finally {
    graphLoading.value = false
  }
}

/** 加载并解析每日实用贴士。 */
async function loadTip() {
  tipLoading.value = true
  tipError.value = ''
  try {
    tip.value = parseDictionaryTip(await getDictionaryDailyTip())
  } catch (error) {
    tip.value = null
    handleContentError(error, (message) => (tipError.value = message))
  } finally {
    tipLoading.value = false
  }
}

/** 并行刷新词典首页的两项精选内容。 */
async function loadDiscovery() {
  await Promise.all([loadGraph(), loadTip()])
}

/** 登录成功后重新请求两项精选内容。 */
async function handleLoginSuccess() {
  loginOpen.value = false
  await loadDiscovery()
}

/** 点击缩略图后按需加载完整图片。 */
async function openGraphPreview() {
  if (!graph.value) return
  previewOpen.value = true
  previewError.value = ''
  if (previewImageUrl.value || previewLoading.value) return

  const requestVersion = ++previewRequestVersion
  previewLoading.value = true
  try {
    const image = await getDictionaryGraphImage(graph.value.file)
    if (requestVersion !== previewRequestVersion) return
    previewImageUrl.value = createImageUrl(image.bytes, image.mimeType)
  } catch (error) {
    if (requestVersion !== previewRequestVersion) return
    previewError.value = getReadableCommandError(error)
  } finally {
    if (requestVersion === previewRequestVersion) previewLoading.value = false
  }
}

function closeGraphPreview() {
  previewOpen.value = false
}

onMounted(loadDiscovery)
onUnmounted(() => {
  previewRequestVersion += 1
  revokeObjectUrl(thumbnailUrl.value)
  revokeObjectUrl(previewImageUrl.value)
})
</script>

<template>
  <section class="dictionary-discovery" aria-label="词典精选内容">
    <a-card class="feature-card" :bordered="true">
      <template #title>
        <span class="feature-title">图解词汇</span>
      </template>
      <div v-if="graphLoading" class="feature-loading"><a-spin /></div>
      <button
        v-else-if="graph"
        type="button"
        class="feature-action graph-action"
        :aria-label="`全屏查看 ${graph.english} ${graph.chinese} 图解词汇`"
        @click="openGraphPreview"
      >
        <img :src="thumbnailUrl" :alt="`${graph.english} ${graph.chinese}`" />
        <span class="graph-caption">
          <strong>{{ graph.english }}</strong>
          <span>{{ graph.chinese }}</span>
        </span>
      </button>
      <div v-else class="feature-error">
        <span>{{ graphError }}</span>
        <a-button v-if="!loginOpen" type="link" @click="loadGraph">重试</a-button>
      </div>
    </a-card>

    <a-card class="feature-card" :bordered="true">
      <template #title>
        <span class="feature-title">实用贴士</span>
      </template>
      <div v-if="tipLoading" class="feature-loading"><a-spin /></div>
      <button
        v-else-if="tip"
        type="button"
        class="feature-action tip-action"
        :aria-label="`查看 ${tip.word} 实用贴士详情`"
        @click="emit('open-tip', tip)"
      >
        <strong class="tip-word">{{ tip.word }}</strong>
        <span class="tip-subtitle">{{ tip.subtitle }}</span>
        <span class="tip-category">{{ tip.category }}</span>
      </button>
      <div v-else class="feature-error">
        <span>{{ tipError }}</span>
        <a-button v-if="!loginOpen" type="link" @click="loadTip">重试</a-button>
      </div>
    </a-card>

    <DictionaryGraphPreview
      :open="previewOpen"
      :image-url="previewImageUrl"
      :loading="previewLoading"
      :error="previewError"
      :alt="graph ? `${graph.english} ${graph.chinese}` : '图解词汇'"
      @close="closeGraphPreview"
    />
    <DictionaryLoginModal
      :open="loginOpen"
      @cancel="loginOpen = false"
      @success="handleLoginSuccess"
    />
  </section>
</template>

<style scoped>
.dictionary-discovery {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  width: min(calc(100% - 48px), 620px);
  margin: 4px auto 32px;
}
.feature-card {
  min-width: 0;
  overflow: hidden;
}
.feature-card :deep(.ant-card-head) {
  min-height: 46px;
  padding: 0 18px;
}
.feature-card :deep(.ant-card-body) {
  min-height: 150px;
  padding: 10px 18px 16px;
}
.feature-title {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-weight: 500;
}
.feature-title::before {
  width: 4px;
  height: 14px;
  background: #d9485f;
  content: '';
}
.feature-action {
  display: flex;
  width: 100%;
  min-height: 124px;
  align-items: center;
  justify-content: center;
  border: 0;
  padding: 0;
  color: var(--ant-color-text);
  background: transparent;
  cursor: pointer;
  transition: background-color 160ms ease;
}
.feature-action:hover {
  background: var(--ant-color-fill-quaternary);
}
.feature-action:focus-visible {
  outline: 2px solid var(--ant-color-primary);
  outline-offset: 3px;
}
.graph-action {
  flex-direction: column;
  gap: 8px;
}
.graph-action img {
  display: block;
  width: min(100%, 160px);
  height: 90px;
  border: 1px solid var(--ant-color-border-secondary);
  border-radius: 4px;
  object-fit: cover;
}
.graph-caption {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.graph-caption strong {
  font-size: 16px;
}
.graph-caption span,
.tip-subtitle,
.tip-category {
  color: var(--ant-color-text-secondary);
}
.tip-action {
  flex-direction: column;
  gap: 10px;
  font-family: system-ui, sans-serif;
}
.tip-word {
  font:
    700 30px Georgia,
    'Times New Roman',
    serif;
}
.feature-loading,
.feature-error {
  display: flex;
  min-height: 124px;
  align-items: center;
  justify-content: center;
}
.feature-error {
  flex-direction: column;
  color: var(--ant-color-text-secondary);
  text-align: center;
}
@media (max-width: 419px) {
  .dictionary-discovery {
    grid-template-columns: 1fr;
    width: calc(100% - 28px);
  }
}
@media (prefers-reduced-motion: reduce) {
  .feature-action {
    transition: none;
  }
}
</style>
