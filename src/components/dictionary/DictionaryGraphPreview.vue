<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { theme } from 'ant-design-vue'
import {
  ArrowLeftOutlined,
  FullscreenExitOutlined,
  ZoomInOutlined,
  ZoomOutOutlined,
} from '@ant-design/icons-vue'

const props = defineProps<{
  open: boolean
  imageUrl: string
  loading: boolean
  error: string
  alt: string
}>()

const emit = defineEmits<{ (event: 'close'): void }>()
const { token } = theme.useToken()
const zoomLevel = ref(1)
const pinchStartDistance = ref<number | null>(null)
const pinchStartZoom = ref(1)
const previewContentRef = ref<HTMLElement | null>(null)
const isDragging = ref(false)
const dragStart = ref<{
  pointerId: number
  clientX: number
  clientY: number
  scrollLeft: number
  scrollTop: number
} | null>(null)
const MIN_ZOOM = 0.25
const MAX_ZOOM = 5
const BUTTON_ZOOM_STEP = 0.25
const WHEEL_ZOOM_STEP = 0.1
const previewStyle = computed(() => ({
  '--dictionary-preview-background': token.value.colorBgBase,
  '--dictionary-preview-text': token.value.colorText,
}))
const imageStyle = computed(() => ({ width: `${zoomLevel.value * 100}%` }))
const zoomPercentage = computed(() => `${Math.round(zoomLevel.value * 100)}%`)
const canZoomOut = computed(() => Boolean(props.imageUrl) && zoomLevel.value > MIN_ZOOM)
const canZoomIn = computed(() => Boolean(props.imageUrl) && zoomLevel.value < MAX_ZOOM)

/** 将图片缩放限制在允许范围内。
 * @param level - 目标缩放比例。
 */
function setZoomLevel(level: number) {
  zoomLevel.value = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, level))
}

/** 放大图片。 */
function zoomIn() {
  setZoomLevel(zoomLevel.value + BUTTON_ZOOM_STEP)
}

/** 缩小图片。 */
function zoomOut() {
  setZoomLevel(zoomLevel.value - BUTTON_ZOOM_STEP)
}

/** 将图片和显示位置恢复到初始状态。 */
function resetView() {
  setZoomLevel(1)
  resetDragState()
  void nextTick(() => {
    if (!previewContentRef.value) return
    previewContentRef.value.scrollLeft = 0
    previewContentRef.value.scrollTop = 0
  })
}

/** 处理桌面端组合键滚轮缩放。
 * @param event - 图片滚动区域的滚轮事件。
 */
function handleWheel(event: WheelEvent) {
  if ((!event.ctrlKey && !event.metaKey) || !props.imageUrl) return
  event.preventDefault()
  setZoomLevel(zoomLevel.value + (event.deltaY < 0 ? WHEEL_ZOOM_STEP : -WHEEL_ZOOM_STEP))
}

/** 计算两个触点之间的距离。
 * @param touches - 当前触点列表。
 */
function getTouchDistance(touches: TouchList) {
  if (touches.length < 2) return 0
  const deltaX = touches[0].clientX - touches[1].clientX
  const deltaY = touches[0].clientY - touches[1].clientY
  return Math.hypot(deltaX, deltaY)
}

/** 记录双指缩放开始时的距离和比例。
 * @param event - 触摸开始事件。
 */
function handleTouchStart(event: TouchEvent) {
  if (event.touches.length !== 2 || !props.imageUrl) return
  pinchStartDistance.value = getTouchDistance(event.touches)
  pinchStartZoom.value = zoomLevel.value
}

/** 根据双指距离更新图片比例。
 * @param event - 触摸移动事件。
 */
function handleTouchMove(event: TouchEvent) {
  if (event.touches.length !== 2 || !pinchStartDistance.value) return
  event.preventDefault()
  setZoomLevel(pinchStartZoom.value * (getTouchDistance(event.touches) / pinchStartDistance.value))
}

/** 清理双指缩放状态。 */
function handleTouchEnd() {
  pinchStartDistance.value = null
}

/** 清理图片拖拽状态。 */
function resetDragState() {
  isDragging.value = false
  dragStart.value = null
}

/** 开始使用鼠标或触控笔拖动图片。
 * @param event - 指针按下事件。
 */
function handlePointerDown(event: PointerEvent) {
  const container = event.currentTarget as HTMLElement
  const hasOverflow =
    container.scrollWidth > container.clientWidth || container.scrollHeight > container.clientHeight
  if (event.pointerType === 'touch' || event.button !== 0 || !props.imageUrl || !hasOverflow) {
    return
  }

  event.preventDefault()
  container.setPointerCapture?.(event.pointerId)
  isDragging.value = true
  dragStart.value = {
    pointerId: event.pointerId,
    clientX: event.clientX,
    clientY: event.clientY,
    scrollLeft: container.scrollLeft,
    scrollTop: container.scrollTop,
  }
}

/** 根据指针移动距离平移图片显示位置。
 * @param event - 指针移动事件。
 */
function handlePointerMove(event: PointerEvent) {
  const start = dragStart.value
  if (!start || start.pointerId !== event.pointerId) return

  event.preventDefault()
  const container = event.currentTarget as HTMLElement
  container.scrollLeft = start.scrollLeft - (event.clientX - start.clientX)
  container.scrollTop = start.scrollTop - (event.clientY - start.clientY)
}

/** 结束图片拖拽并释放指针捕获。
 * @param event - 指针结束或取消事件。
 */
function handlePointerEnd(event: PointerEvent) {
  const start = dragStart.value
  if (!start || start.pointerId !== event.pointerId) return

  const container = event.currentTarget as HTMLElement
  if (container.hasPointerCapture?.(event.pointerId)) {
    container.releasePointerCapture?.(event.pointerId)
  }
  resetDragState()
}

watch(
  () => [props.open, props.imageUrl] as const,
  ([open]) => {
    if (open) resetView()
  },
)

/** 将预览限制在应用标题栏下方的内容区域。 */
function getPreviewContainer(): HTMLElement {
  return document.querySelector<HTMLElement>('.app-main-container') ?? document.body
}
</script>

<template>
  <a-modal
    :open="open"
    :footer="null"
    :width="'100%'"
    :get-container="getPreviewContainer"
    :mask="false"
    :closable="false"
    :style="previewStyle"
    wrap-class-name="dictionary-graph-preview"
    destroy-on-close
    @cancel="emit('close')"
  >
    <div class="preview-layout">
      <header class="preview-toolbar">
        <a-button class="preview-back" type="text" aria-label="返回图解词汇" @click="emit('close')">
          <template #icon><ArrowLeftOutlined /></template>
          返回
        </a-button>

        <div class="preview-zoom-controls" aria-label="图片缩放控制">
          <a-button
            class="preview-zoom-button"
            type="text"
            aria-label="缩小图片"
            :disabled="!canZoomOut"
            @click="zoomOut"
          >
            <template #icon><ZoomOutOutlined /></template>
          </a-button>
          <span class="preview-zoom-percentage" aria-live="polite">{{ zoomPercentage }}</span>
          <a-button
            class="preview-zoom-button"
            type="text"
            aria-label="放大图片"
            :disabled="!canZoomIn"
            @click="zoomIn"
          >
            <template #icon><ZoomInOutlined /></template>
          </a-button>
          <a-button
            class="preview-zoom-button"
            type="text"
            aria-label="恢复图片大小和位置"
            :disabled="!imageUrl"
            @click="resetView"
          >
            <template #icon><FullscreenExitOutlined /></template>
          </a-button>
        </div>
      </header>

      <div
        ref="previewContentRef"
        class="preview-content"
        :class="{ 'has-image': imageUrl, 'is-dragging': isDragging }"
        @wheel="handleWheel"
        @pointerdown="handlePointerDown"
        @pointermove="handlePointerMove"
        @pointerup="handlePointerEnd"
        @pointercancel="handlePointerEnd"
        @touchstart="handleTouchStart"
        @touchmove="handleTouchMove"
        @touchend="handleTouchEnd"
        @touchcancel="handleTouchEnd"
      >
        <a-spin v-if="loading" size="large" tip="正在加载完整图片…" />
        <a-alert v-else-if="error" type="error" show-icon :message="error" />
        <div v-else-if="imageUrl" class="preview-image-stage">
          <img :src="imageUrl" :alt="alt" :style="imageStyle" :draggable="false" />
        </div>
      </div>
    </div>
  </a-modal>
</template>

<style scoped>
.preview-layout {
  display: flex;
  height: 100%;
  min-height: 0;
  flex-direction: column;
}
.preview-toolbar {
  display: flex;
  min-height: 52px;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  padding: 6px 16px;
  border-bottom: 1px solid var(--ant-color-border-secondary);
  background: var(--dictionary-preview-background);
}
.preview-back {
  display: inline-flex;
  align-items: center;
  color: var(--dictionary-preview-text);
}
.preview-zoom-controls {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}
.preview-zoom-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--dictionary-preview-text);
}
.preview-zoom-percentage {
  min-width: 52px;
  color: var(--dictionary-preview-text);
  text-align: center;
  font-variant-numeric: tabular-nums;
}
.preview-content {
  display: grid;
  flex: 1;
  min-height: 0;
  place-items: center;
  overflow: auto;
  padding: 24px;
  background: var(--dictionary-preview-background);
  overscroll-behavior: contain;
  touch-action: pan-x pan-y;
  user-select: none;
}
.preview-content.has-image {
  cursor: grab;
}
.preview-content.is-dragging {
  cursor: grabbing;
}
.preview-image-stage {
  display: flex;
  width: 100%;
  min-width: 100%;
  min-height: 100%;
  align-items: flex-start;
  justify-content: center;
}
.preview-image-stage img {
  display: block;
  height: auto;
  max-width: none;
  flex: 0 0 auto;
}
:global(.dictionary-graph-preview) {
  position: absolute !important;
  inset: 0 !important;
  overflow: hidden;
}
:global(.dictionary-graph-preview .ant-modal) {
  top: 0;
  width: 100% !important;
  height: 100%;
  max-width: none;
  margin: 0;
  padding: 0;
}
:global(.dictionary-graph-preview .ant-modal-content) {
  box-sizing: border-box;
  height: 100%;
  border-radius: 0;
  padding: 0;
  background: var(--dictionary-preview-background);
}
:global(.dictionary-graph-preview .ant-modal-body) {
  height: 100%;
}
</style>
