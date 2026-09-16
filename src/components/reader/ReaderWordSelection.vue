<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch, nextTick } from 'vue'
import { message, theme } from 'ant-design-vue'
import { CopyOutlined, SearchOutlined } from '@ant-design/icons-vue'
import type { BookMetadata } from '@/types'
import {
  copyReaderWord,
  findReaderWord,
  recognizeReaderPage,
  type RecognizedPage,
} from '@/lib/api/readerOcr'

const props = defineProps<{
  productCode: string
  pages: Array<{ label: string; url: string }>
  metadata: BookMetadata | null
  zoom: number
  disabled: boolean
}>()
const emit = defineEmits<{
  (event: 'query', word: string): void
  (event: 'selection-change', selected: boolean): void
}>()
const layer = ref<HTMLElement | null>(null)
const { token } = theme.useToken()
const selection = ref<{
  text: string
  x: number
  y: number
  width: number
  height: number
  menuX: number
  menuY: number
} | null>(null)
let host: HTMLElement | null = null
let generation = 0
let disposed = false
let busy = false
let longPress: ReturnType<typeof setTimeout> | null = null
let start: { x: number; y: number; pointerId: number } | null = null
let suppressClick = false
let lastPointerType = 'mouse'
const results = new Map<string, RecognizedPage>()
const failures = new Map<string, string>()

/** 取消尚未成立的长按，不影响现有触摸翻页。 */
function cancelPress() {
  if (longPress !== null) clearTimeout(longPress)
  longPress = null
  start = null
}

/** 清理当前选区及菜单。 */
function clearSelection() {
  cancelPress()
  selection.value = null
  emit('selection-change', false)
}

/** 查找加载完成的书页图像。 */
function imageFor(label: string): HTMLImageElement | undefined {
  return Array.from(host?.querySelectorAll<HTMLImageElement>('.page-surface img') || []).find(
    (image) =>
      image.closest<HTMLElement>('[data-page-label]')?.dataset.pageLabel === label &&
      image.complete &&
      image.naturalWidth > 0,
  )
}

/** 串行预热可见页；翻页后的迟到结果不再写入本地状态。 */
async function warmPages() {
  if (busy || disposed || !props.productCode) return
  const page = props.pages.find(
    (item) =>
      item.url && imageFor(item.label) && !results.has(item.label) && !failures.has(item.label),
  )
  if (!page) return
  const version = generation
  busy = true
  try {
    const result = await recognizeReaderPage(props.productCode, page.label)
    if (!disposed && version === generation) results.set(page.label, result)
  } catch (error) {
    if (!disposed && version === generation)
      failures.set(page.label, error instanceof Error ? error.message : String(error))
  } finally {
    busy = false
    if (!disposed) void warmPages()
  }
}

/** 从真实渲染矩形命中单词，热点矩形沿用书页元数据。 */
function selectAt(x: number, y: number, target: EventTarget | null) {
  if (props.disabled || !(target instanceof Element) || target.closest('.overlay-item')) return
  const surface = target.closest<HTMLElement>('.page-surface')
  const label = surface?.dataset.pageLabel
  const image = label ? imageFor(label) : undefined
  if (!label || !image || !host) return
  const data = results.get(label)
  if (!data) {
    const failure = failures.get(label)
    message.info(failure || '正在识别本页文字，请稍候重试')
    if (failure) failures.delete(label)
    void warmPages()
    return
  }
  const rect = image.getBoundingClientRect()
  if (!rect.width || !rect.height) return
  const width = props.metadata?.pageWidth || 1
  const height = props.metadata?.pageHeight || 1
  const hotspots = (props.metadata?.pages[label]?.overlays || []).map((item) => ({
    x: item.x / width,
    y: item.y / height,
    width: item.w / width,
    height: item.h / height,
  }))
  const word = findReaderWord(
    data,
    (x - rect.left) / rect.width,
    (y - rect.top) / rect.height,
    hotspots,
  )
  if (!word) return
  const root = host.getBoundingClientRect()
  const wordX = rect.left + word.rect.x * rect.width - root.left
  const wordY = rect.top + word.rect.y * rect.height - root.top
  selection.value = {
    text: word.text,
    x: wordX,
    y: wordY,
    width: word.rect.width * rect.width,
    height: word.rect.height * rect.height,
    menuX: Math.max(8, Math.min(wordX, root.width - 184)),
    menuY: Math.max(
      8,
      Math.min(
        wordY >= 60 ? wordY - 56 : wordY + word.rect.height * rect.height + 8,
        root.height - 56,
      ),
    ),
  }
  emit('selection-change', true)
}

/** 非热点的手指或 Pencil 长按；双指操作立即放弃选词。 */
function pointerDown(event: PointerEvent) {
  lastPointerType = event.pointerType || 'mouse'
  if ((event.target as Element)?.closest('.reader-word-actions')) return
  const alreadyPressed = start !== null
  clearSelection()
  suppressClick = false
  if (
    alreadyPressed ||
    props.disabled ||
    event.isPrimary === false ||
    !['touch', 'pen'].includes(event.pointerType)
  )
    return
  if (
    !(event.target instanceof Element) ||
    !event.target.closest('.page-surface') ||
    event.target.closest('.overlay-item')
  )
    return
  start = { x: event.clientX, y: event.clientY, pointerId: event.pointerId }
  longPress = setTimeout(() => {
    longPress = null
    selectAt(event.clientX, event.clientY, event.target)
    suppressClick = selection.value !== null
  }, 450)
}

/** 移动取消待触发长按，已选词手势也不用于翻页。 */
function pointerMove(event: PointerEvent) {
  if (start && Math.hypot(event.clientX - start.x, event.clientY - start.y) > 8) cancelPress()
}

/** 鼠标双击仅处理非热点。 */
function doubleClick(event: MouseEvent) {
  if (lastPointerType !== 'mouse') return
  if ((event.target as Element)?.closest('.reader-word-actions')) return
  clearSelection()
  selectAt(event.clientX, event.clientY, event.target)
}

/** 已完成长按时吞掉该次模拟点击。 */
function click(event: MouseEvent) {
  if (suppressClick && !(event.target as Element)?.closest('.reader-word-actions')) {
    event.preventDefault()
    event.stopPropagation()
    suppressClick = false
  }
}

/** Escape 优先关闭选词菜单。 */
function keydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && selection.value) {
    event.preventDefault()
    event.stopImmediatePropagation()
    clearSelection()
  }
}

/** 点击阅读器外部控件时释放选区，不占用后续快捷键。 */
function outsidePointerDown(event: PointerEvent) {
  if (event.target instanceof Node && !host?.contains(event.target)) clearSelection()
}

/** 复制成功后关闭菜单；失败允许重试。 */
async function copy() {
  const selected = selection.value
  if (!selected) return
  try {
    await copyReaderWord(selected.text)
    if (selection.value === selected) clearSelection()
    message.success('已复制')
  } catch (error) {
    message.error(String(error))
  }
}

/** 固定本次查询文本，再清理选区。 */
function query() {
  const word = selection.value?.text
  clearSelection()
  if (word) emit('query', word)
}

watch(
  () => [props.productCode, props.pages],
  async () => {
    generation++
    clearSelection()
    results.clear()
    failures.clear()
    await nextTick()
    void warmPages()
  },
  { deep: true },
)
watch(() => [props.zoom, props.disabled], clearSelection)

onMounted(() => {
  host = layer.value?.parentElement || null
  host?.addEventListener('pointerdown', pointerDown, true)
  host?.addEventListener('pointermove', pointerMove, true)
  host?.addEventListener('pointerup', cancelPress, true)
  host?.addEventListener('pointercancel', cancelPress, true)
  host?.addEventListener('dblclick', doubleClick, true)
  host?.addEventListener('click', click, true)
  host?.addEventListener('load', warmPages, true)
  host?.addEventListener('scroll', clearSelection, true)
  window.addEventListener('keydown', keydown, true)
  window.addEventListener('resize', clearSelection)
  window.addEventListener('pointerdown', outsidePointerDown, true)
  window.visualViewport?.addEventListener('resize', clearSelection)
  void warmPages()
})
onUnmounted(() => {
  disposed = true
  generation++
  clearSelection()
  host?.removeEventListener('pointerdown', pointerDown, true)
  host?.removeEventListener('pointermove', pointerMove, true)
  host?.removeEventListener('pointerup', cancelPress, true)
  host?.removeEventListener('pointercancel', cancelPress, true)
  host?.removeEventListener('dblclick', doubleClick, true)
  host?.removeEventListener('click', click, true)
  host?.removeEventListener('load', warmPages, true)
  host?.removeEventListener('scroll', clearSelection, true)
  window.removeEventListener('keydown', keydown, true)
  window.removeEventListener('resize', clearSelection)
  window.removeEventListener('pointerdown', outsidePointerDown, true)
  window.visualViewport?.removeEventListener('resize', clearSelection)
})
</script>

<template>
  <div ref="layer" class="reader-word-layer">
    <template v-if="selection">
      <div
        class="reader-word-highlight"
        :style="{
          left: `${selection.x}px`,
          top: `${selection.y}px`,
          width: `${selection.width}px`,
          height: `${selection.height}px`,
          background: token.colorPrimaryBg,
          outlineColor: token.colorPrimary,
        }"
      />
      <div
        class="reader-word-actions"
        role="toolbar"
        aria-label="单词操作"
        :style="{
          left: `${selection.menuX}px`,
          top: `${selection.menuY}px`,
          background: token.colorBgElevated,
          boxShadow: token.boxShadowSecondary,
        }"
      >
        <a-button type="text" @click="copy"
          ><template #icon><CopyOutlined /></template>复制</a-button
        >
        <a-button type="text" @click="query"
          ><template #icon><SearchOutlined /></template>查询</a-button
        >
      </div>
    </template>
  </div>
</template>

<style scoped>
.reader-word-layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: 40;
  overflow: hidden;
}
.reader-word-highlight {
  position: absolute;
  opacity: 0.45;
  outline: 1px solid;
  border-radius: 2px;
}
.reader-word-actions {
  position: absolute;
  pointer-events: auto;
  display: flex;
  padding: 2px;
  border-radius: 8px;
}
.reader-word-actions :deep(.ant-btn) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 44px;
}
.reader-word-actions :deep(.anticon) {
  display: inline-flex;
  align-items: center;
}
.reader-word-actions :deep(svg) {
  display: block;
}
</style>
