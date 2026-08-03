<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { storeToRefs } from 'pinia'
import { useReaderStore } from '../../stores/reader'
import type { BookMetadata } from '../../types'
import LoadingOverlay from '../common/loading/LoadingOverlay.vue'
import { CustomerServiceOutlined, LinkOutlined, KeyOutlined } from '@ant-design/icons-vue'

const props = defineProps<{
  metadata: BookMetadata | null
  loading: boolean
  leftPageUrl: string
  rightPageUrl: string
  leftPageLabel: string
  rightPageLabel: string
  showHotspots: boolean
  canGoBack: boolean
  canGoForward: boolean
}>()

const emit = defineEmits<{
  (e: 'overlayClick', overlay: any): void
  (e: 'goBack'): void
  (e: 'goForward'): void
}>()
const { t } = useI18n()

const readerStore = useReaderStore()
const { viewMode, zoomLevel } = storeToRefs(readerStore)
const scrollContainerRef = ref<HTMLElement | null>(null)
const pinchStartDistance = ref<number | null>(null)
const pinchStartZoom = ref(1)
const pinchLastDistance = ref<number | null>(null)
const pinchPendingZoom = ref<number | null>(null)
const pinchDebounceTimer = ref<number | null>(null)
const swipeStartPoint = ref<{ x: number; y: number } | null>(null)
const swipeDirectionLock = ref<'horizontal' | 'vertical' | null>(null)
const swipeTriggered = ref(false)
const gestureEvents = ['gesturestart', 'gesturechange', 'gestureend'] as const
const PINCH_DEBOUNCE_MS = 16
const PINCH_DISTANCE_DEADZONE = 2
const PINCH_SCALE_SENSITIVITY = 0.85
const PINCH_SMOOTHING_FACTOR = 0.35
const SWIPE_LOCK_THRESHOLD = 10
const SWIPE_TRIGGER_DISTANCE = 56
const SWIPE_MAX_VERTICAL_DRIFT = 48

function handleWheel(e: WheelEvent) {
  if (e.ctrlKey || e.metaKey) {
    e.preventDefault()
    if (e.deltaY < 0) {
      readerStore.zoomIn(0.02)
    } else {
      readerStore.zoomOut(0.02)
    }
  }
}

function getTouchDistance(touches: TouchList) {
  if (touches.length < 2) return 0
  const dx = touches[0].clientX - touches[1].clientX
  const dy = touches[0].clientY - touches[1].clientY
  return Math.hypot(dx, dy)
}

function clearPinchDebounceTimer() {
  if (pinchDebounceTimer.value !== null) {
    window.clearTimeout(pinchDebounceTimer.value)
    pinchDebounceTimer.value = null
  }
}

/** 应用合并后的双指缩放值，限制更新频率和缩放边界。
 * @param force - 是否忽略防抖间隔立即应用。
 */
function applyPendingPinchZoom(force = false) {
  if (pinchPendingZoom.value === null) return

  const targetZoom = pinchPendingZoom.value
  pinchPendingZoom.value = null

  if (force) {
    readerStore.setZoomLevel(targetZoom)
    return
  }

  const delta = targetZoom - zoomLevel.value
  if (Math.abs(delta) < 0.001) {
    readerStore.setZoomLevel(targetZoom)
    return
  }

  readerStore.setZoomLevel(zoomLevel.value + delta * PINCH_SMOOTHING_FACTOR)
}

/** 合并连续双指缩放事件并延迟提交最终缩放值。
 * @param targetZoom - 本次手势计算出的目标缩放值。
 */
function queuePinchZoom(targetZoom: number) {
  pinchPendingZoom.value = targetZoom
  if (pinchDebounceTimer.value !== null) return

  pinchDebounceTimer.value = window.setTimeout(() => {
    pinchDebounceTimer.value = null
    applyPendingPinchZoom()
  }, PINCH_DEBOUNCE_MS)
}

function resetPinchState() {
  pinchStartDistance.value = null
  pinchLastDistance.value = null
  pinchPendingZoom.value = null
  clearPinchDebounceTimer()
}

function resetSwipeState() {
  swipeStartPoint.value = null
  swipeDirectionLock.value = null
  swipeTriggered.value = false
}

function isAppleTouchDevice() {
  if (typeof navigator === 'undefined') return false
  const ua = navigator.userAgent || ''
  const isLegacyIos = /iPad|iPhone|iPod/i.test(ua)
  const isIpadOs = navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1
  return isLegacyIos || isIpadOs
}

/** 判断当前设备和缩放状态是否允许滑动翻页。 */
function shouldHandleSwipeNavigation() {
  return isAppleTouchDevice() && zoomLevel.value <= 1.02
}

/** 初始化双指缩放或单指滑动手势状态。
 * @param e - 触摸开始事件。
 */
function handleTouchStart(e: TouchEvent) {
  if (e.touches.length === 1) {
    const touch = e.touches[0]
    swipeStartPoint.value = { x: touch.clientX, y: touch.clientY }
    swipeDirectionLock.value = null
    swipeTriggered.value = false
  } else {
    resetSwipeState()
  }

  if (e.touches.length !== 2) return
  pinchStartDistance.value = getTouchDistance(e.touches)
  pinchStartZoom.value = zoomLevel.value
  pinchLastDistance.value = pinchStartDistance.value
  pinchPendingZoom.value = null
  clearPinchDebounceTimer()
}

/** 更新缩放或滑动手势，并阻止冲突的浏览器默认行为。
 * @param e - 触摸移动事件。
 */
function handleTouchMove(e: TouchEvent) {
  if (e.touches.length === 1 && swipeStartPoint.value && !pinchStartDistance.value) {
    if (!shouldHandleSwipeNavigation()) return

    const touch = e.touches[0]
    const deltaX = touch.clientX - swipeStartPoint.value.x
    const deltaY = touch.clientY - swipeStartPoint.value.y
    const absDeltaX = Math.abs(deltaX)
    const absDeltaY = Math.abs(deltaY)

    if (
      !swipeDirectionLock.value &&
      (absDeltaX > SWIPE_LOCK_THRESHOLD || absDeltaY > SWIPE_LOCK_THRESHOLD)
    ) {
      swipeDirectionLock.value = absDeltaX > absDeltaY ? 'horizontal' : 'vertical'
    }

    if (swipeDirectionLock.value !== 'horizontal') return
    if (absDeltaY > SWIPE_MAX_VERTICAL_DRIFT) return

    // 用户滑动翻页时阻止 iOS 同步触发页面横向滚动。
    e.preventDefault()

    if (swipeTriggered.value || absDeltaX < SWIPE_TRIGGER_DISTANCE) return

    if (deltaX < 0 && props.canGoForward) {
      emit('goForward')
      swipeTriggered.value = true
    } else if (deltaX > 0 && props.canGoBack) {
      emit('goBack')
      swipeTriggered.value = true
    }
    return
  }

  if (e.touches.length !== 2 || !pinchStartDistance.value) return

  const distance = getTouchDistance(e.touches)
  if (!distance) return
  if (pinchLastDistance.value !== null) {
    const distanceDelta = Math.abs(distance - pinchLastDistance.value)
    if (distanceDelta < PINCH_DISTANCE_DEADZONE) return
  }

  e.preventDefault()
  pinchLastDistance.value = distance

  const pinchRatio = distance / pinchStartDistance.value
  const adjustedRatio = 1 + (pinchRatio - 1) * PINCH_SCALE_SENSITIVITY
  queuePinchZoom(pinchStartZoom.value * adjustedRatio)
}

/** 结束当前手势，并在达到阈值时触发翻页。
 * @param e - 触摸结束事件。
 */
function handleTouchEnd(e: TouchEvent) {
  if (e.touches.length < 2) {
    applyPendingPinchZoom(true)
    resetPinchState()
  }
  if (e.touches.length === 0) {
    resetSwipeState()
  }
}

function handleTouchCancel() {
  resetPinchState()
  resetSwipeState()
}

function preventNativePinchZoom(e: Event) {
  e.preventDefault()
}

onMounted(() => {
  const container = scrollContainerRef.value
  if (!container) return

  for (const eventName of gestureEvents) {
    container.addEventListener(eventName, preventNativePinchZoom, { passive: false })
  }
})

onUnmounted(() => {
  const container = scrollContainerRef.value
  clearPinchDebounceTimer()
  if (!container) return

  for (const eventName of gestureEvents) {
    container.removeEventListener(eventName, preventNativePinchZoom)
  }
})

const pageSurfaceStyle = computed(() => {
  if (!props.metadata) return {}
  const ratio = props.metadata.pageWidth / props.metadata.pageHeight
  const isSingleView = viewMode.value === 'single'
  return {
    aspectRatio: `${ratio}`,
    width: isSingleView ? '100%' : '0',
    maxWidth: isSingleView ? '100%' : undefined,
    flex: isSingleView ? '1 1 100%' : '1 1 0',
    height: 'auto',
    flexShrink: 0,
  }
})

const imageStyle = computed(() => ({
  width: '100%',
  height: 'auto',
  display: 'block',
}))

function getOverlayStyle(overlay: any) {
  if (!props.metadata) return {}
  const pw = props.metadata.pageWidth || 1
  const ph = props.metadata.pageHeight || 1
  return {
    left: `${(overlay.x / pw) * 100}%`,
    top: `${(overlay.y / ph) * 100}%`,
    width: `${(overlay.w / pw) * 100}%`,
    height: `${(overlay.h / ph) * 100}%`,
  }
}
</script>

<template>
  <div class="relative flex flex-1 flex-col overflow-hidden">
    <LoadingOverlay
      :visible="loading"
      :message="t('app.loading')"
      mode="inline"
      backdrop="soft"
      :z-index="60"
    />

    <div
      ref="scrollContainerRef"
      class="reader-scroll-container pinch-zoom-surface custom-scrollbar flex-1 overflow-auto"
      @wheel="handleWheel"
      @touchstart="handleTouchStart"
      @touchmove="handleTouchMove"
      @touchend="handleTouchEnd"
      @touchcancel="handleTouchCancel"
    >
      <div
        class="reader-content-container flex w-full origin-top items-start justify-center gap-4 px-0 pt-1 transition-transform duration-150 ease-out"
        :style="{ transform: `scale(${zoomLevel})` }"
      >
        <!-- 左页 -->
        <div
          v-if="leftPageLabel"
          class="page-surface relative overflow-hidden bg-white shadow-xl dark:bg-black"
          :style="pageSurfaceStyle"
        >
          <img v-if="leftPageUrl" :src="leftPageUrl" :style="imageStyle" alt="Left Page" />
          <div
            v-if="metadata && metadata.pages[leftPageLabel]"
            class="overlays-layer pointer-events-none absolute inset-0"
            :class="{ 'opacity-0': !showHotspots }"
          >
            <div
              v-for="(overlay, idx) in metadata.pages[leftPageLabel].overlays"
              :key="idx"
              class="overlay-item group pointer-events-auto absolute z-20 flex cursor-pointer items-center justify-center overflow-hidden"
              :style="getOverlayStyle(overlay)"
              @click.stop="emit('overlayClick', overlay)"
            >
              <div
                class="icon-wrapper flex h-9 w-9 items-center justify-center rounded-full"
                :class="{
                  'is-exercise': overlay.type === 'exercise' || overlay.type === 'learning-object',
                }"
              >
                <CustomerServiceOutlined v-if="overlay.type === 'audio'" />
                <KeyOutlined
                  v-else-if="overlay.type === 'exercise' || overlay.type === 'learning-object'"
                />
                <LinkOutlined v-else-if="overlay.type === 'page'" />
              </div>
            </div>
          </div>
          <div
            class="pointer-events-none absolute bottom-2 right-2 rounded bg-black/20 px-2 py-1 text-[10px] text-white"
          >
            {{ leftPageLabel }}
          </div>
        </div>

        <!-- 跨页模式下的空白左页占位 -->
        <div
          v-else-if="viewMode === 'spread'"
          :style="pageSurfaceStyle"
          class="pointer-events-none opacity-0"
        ></div>

        <!-- 右页 -->
        <div
          v-if="viewMode === 'spread' && rightPageLabel"
          class="page-surface relative overflow-hidden bg-white shadow-xl dark:bg-black"
          :style="pageSurfaceStyle"
        >
          <img v-if="rightPageUrl" :src="rightPageUrl" :style="imageStyle" alt="Right Page" />
          <div
            v-if="metadata && metadata.pages[rightPageLabel]"
            class="overlays-layer pointer-events-none absolute inset-0"
            :class="{ 'opacity-0': !showHotspots }"
          >
            <div
              v-for="(overlay, idx) in metadata.pages[rightPageLabel].overlays"
              :key="idx"
              class="overlay-item group pointer-events-auto absolute z-20 flex cursor-pointer items-center justify-center overflow-hidden"
              :style="getOverlayStyle(overlay)"
              @click.stop="emit('overlayClick', overlay)"
            >
              <div
                class="icon-wrapper flex h-9 w-9 items-center justify-center rounded-full"
                :class="{
                  'is-exercise': overlay.type === 'exercise' || overlay.type === 'learning-object',
                }"
              >
                <CustomerServiceOutlined v-if="overlay.type === 'audio'" />
                <KeyOutlined
                  v-else-if="overlay.type === 'exercise' || overlay.type === 'learning-object'"
                />
                <LinkOutlined v-else-if="overlay.type === 'page'" />
              </div>
            </div>
          </div>
          <div
            class="pointer-events-none absolute bottom-2 left-2 rounded bg-black/20 px-2 py-1 text-[10px] text-white"
          >
            {{ rightPageLabel }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pinch-zoom-surface {
  touch-action: pan-x pan-y;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  scroll-padding-bottom: var(--reader-footer-clearance, 96px);
}

.pinch-zoom-surface::-webkit-scrollbar {
  width: 8px;
}

.pinch-zoom-surface::-webkit-scrollbar-track {
  background: transparent;
}

.pinch-zoom-surface::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--ant-color-text-quaternary, #ccc) 30%, transparent);
  border-radius: 10px;
  border: 2px solid transparent;
  background-clip: content-box;
}

.pinch-zoom-surface::-webkit-scrollbar-thumb:hover {
  background: color-mix(in srgb, var(--ant-color-text-quaternary, #999) 60%, transparent);
  background-clip: content-box;
}

.reader-content-container {
  padding-bottom: calc(var(--reader-footer-clearance, 96px) + 8px);
  will-change: transform;
  transition-timing-function: cubic-bezier(0.22, 1, 0.36, 1);
}

.page-surface {
  display: flex;
  justify-content: center;
  align-items: flex-start;
}
.page-surface img {
  user-select: none;
  -webkit-user-drag: none;
}

.overlay-item {
  @apply backdrop-blur-[1px] transition-all duration-300;
  /* 移除边框，改用微弱的投影来界定区域 */
  border: none;
}

/* 图标容器：使用浅色磨砂背景确保深色图标在任何背景下可见 */
.icon-wrapper {
  @apply shadow-sm transition-all duration-300;
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.5);
}

.icon-wrapper.is-exercise {
  background: rgba(224, 242, 254, 0.8); /* 浅蓝色背景 (sky-100) */
  border-color: rgba(186, 230, 253, 0.6);
}

.dark .icon-wrapper.is-exercise {
  background: rgba(7, 89, 133, 0.6); /* 深色模式下的深蓝色 */
  border-color: rgba(12, 74, 110, 0.4);
}

.icon-wrapper.is-exercise .anticon {
  color: #0369a1; /* 深天蓝色图标 */
}

.dark .icon-wrapper.is-exercise .anticon {
  color: #e0f2fe;
}

.dark .icon-wrapper {
  background: rgba(30, 41, 59, 0.8); /* slate-800 */
  border-color: rgba(255, 255, 255, 0.1);
}

/* 统一深色图标 */
.overlay-item .anticon {
  @apply transition-transform duration-300;
  font-size: 20px !important;
  color: #475569; /* slate-600 */
}

.dark .overlay-item .anticon {
  color: #cbd5e1; /* slate-300 */
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.5));
}

.overlay-item:hover .icon-wrapper {
  @apply scale-110 shadow-md;
  background: rgba(255, 255, 255, 0.9);
}

.dark .overlay-item:hover .icon-wrapper {
  background: rgba(51, 65, 85, 0.9); /* slate-700 */
}

.overlay-item:hover {
  @apply z-30;
}
</style>
