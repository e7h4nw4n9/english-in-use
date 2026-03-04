<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  MenuFoldOutlined,
  PauseCircleOutlined,
  PlayCircleOutlined,
  RedoOutlined,
  SaveOutlined,
} from '@ant-design/icons-vue'
import { theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'

const { useToken } = theme
const { token } = useToken()
const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  timerStatus: 'idle' | 'running' | 'paused'
  timerDisplay: string
}>()

const emit = defineEmits<{
  (e: 'timerStart'): void
  (e: 'timerPause'): void
  (e: 'timerResume'): void
  (e: 'timerRestart'): void
  (e: 'timerStopSave'): void
}>()

const EXPANDED_WIDTH = 238
const EXPANDED_HEIGHT = 112
const COLLAPSED_WIDTH = 108
const COLLAPSED_HEIGHT = 40
const VIEWPORT_MARGIN = 12
const DEFAULT_TOP = 84

const isCollapsed = ref(false)
const isDragging = ref(false)
const panelRef = ref<HTMLElement | null>(null)
const dragTarget = ref<HTMLElement | null>(null)
const dragOffset = ref({ x: 0, y: 0 })
const position = ref({ x: 0, y: DEFAULT_TOP })
const lastExpandedPosition = ref({ x: 0, y: DEFAULT_TOP })

const hasTimerSession = computed(() => props.timerStatus !== 'idle')
const timerMainActionLabel = computed(() => {
  if (props.timerStatus === 'running') return t('studyTimer.pause')
  if (props.timerStatus === 'paused') return t('studyTimer.resume')
  return t('studyTimer.start')
})

const panelStyle = computed(() => ({
  left: `${position.value.x}px`,
  top: `${position.value.y}px`,
  '--timer-bg': `${token.value.colorBgElevated}dd`,
  '--timer-border': token.value.colorBorderSecondary,
  '--timer-text': token.value.colorText,
}))

function getViewportSize() {
  return {
    width: window.innerWidth,
    height: window.innerHeight,
  }
}

function getPanelSize(collapsed = isCollapsed.value) {
  return {
    width: panelRef.value?.offsetWidth || (collapsed ? COLLAPSED_WIDTH : EXPANDED_WIDTH),
    height: panelRef.value?.offsetHeight || (collapsed ? COLLAPSED_HEIGHT : EXPANDED_HEIGHT),
  }
}

function clampPosition(next: { x: number; y: number }, collapsed = isCollapsed.value) {
  const { width: viewportWidth, height: viewportHeight } = getViewportSize()
  const { width, height } = getPanelSize(collapsed)
  const minX = VIEWPORT_MARGIN
  const minY = VIEWPORT_MARGIN
  const maxX = Math.max(minX, viewportWidth - width - VIEWPORT_MARGIN)
  const maxY = Math.max(minY, viewportHeight - height - VIEWPORT_MARGIN)

  return {
    x: Math.min(Math.max(next.x, minX), maxX),
    y: Math.min(Math.max(next.y, minY), maxY),
  }
}

async function initPosition() {
  await nextTick()
  const { width } = getPanelSize(false)
  const next = clampPosition(
    {
      x: window.innerWidth - width - VIEWPORT_MARGIN,
      y: DEFAULT_TOP,
    },
    false,
  )
  position.value = next
  lastExpandedPosition.value = next
}

function triggerMainAction() {
  if (props.timerStatus === 'running') {
    emit('timerPause')
    return
  }
  if (props.timerStatus === 'paused') {
    emit('timerResume')
    return
  }
  emit('timerStart')
}

function collapsePanel() {
  isCollapsed.value = true
}

function expandPanel() {
  isCollapsed.value = false
}

function removeDragListeners() {
  if (!dragTarget.value) return

  dragTarget.value.removeEventListener('pointermove', onDragMove as EventListener)
  dragTarget.value.removeEventListener('pointerup', onDragEnd as EventListener)
  dragTarget.value.removeEventListener('pointercancel', onDragEnd as EventListener)
  dragTarget.value = null
}

function onDragStart(event: PointerEvent) {
  if (isCollapsed.value) return

  isDragging.value = true
  dragOffset.value = {
    x: event.clientX - position.value.x,
    y: event.clientY - position.value.y,
  }

  const target = event.currentTarget as HTMLElement
  dragTarget.value = target

  if (typeof target.setPointerCapture === 'function') {
    target.setPointerCapture(event.pointerId)
  }

  target.addEventListener('pointermove', onDragMove as EventListener)
  target.addEventListener('pointerup', onDragEnd as EventListener)
  target.addEventListener('pointercancel', onDragEnd as EventListener)
}

function onDragMove(event: PointerEvent) {
  if (!isDragging.value || isCollapsed.value) return

  const next = clampPosition(
    {
      x: event.clientX - dragOffset.value.x,
      y: event.clientY - dragOffset.value.y,
    },
    false,
  )
  position.value = next
  lastExpandedPosition.value = next
}

function onDragEnd(event: PointerEvent) {
  if (!isDragging.value) return
  isDragging.value = false

  const target = event.currentTarget as HTMLElement
  if (typeof target.releasePointerCapture === 'function') {
    target.releasePointerCapture(event.pointerId)
  }
  removeDragListeners()
}

async function syncPositionForCollapse(nextCollapsed: boolean) {
  if (nextCollapsed) {
    lastExpandedPosition.value = clampPosition(position.value, false)
    await nextTick()
    position.value = clampPosition(
      {
        x: window.innerWidth - getPanelSize(true).width - VIEWPORT_MARGIN,
        y: position.value.y,
      },
      true,
    )
    return
  }

  await nextTick()
  position.value = clampPosition(lastExpandedPosition.value, false)
}

function handleViewportResize() {
  position.value = clampPosition(position.value)
  if (!isCollapsed.value) {
    lastExpandedPosition.value = clampPosition(lastExpandedPosition.value, false)
  }
}

watch(isCollapsed, (nextCollapsed) => {
  if (!props.visible) return
  void syncPositionForCollapse(nextCollapsed)
})

watch(
  () => props.visible,
  (visible) => {
    if (!visible) {
      isDragging.value = false
      removeDragListeners()
      return
    }
    isCollapsed.value = false
    void nextTick().then(() => {
      handleViewportResize()
    })
  },
)

onMounted(() => {
  void initPosition()
  window.addEventListener('resize', handleViewportResize)
  window.addEventListener('orientationchange', handleViewportResize)
})

onBeforeUnmount(() => {
  removeDragListeners()
  window.removeEventListener('resize', handleViewportResize)
  window.removeEventListener('orientationchange', handleViewportResize)
})
</script>

<template>
  <div v-if="visible" ref="panelRef" class="study-timer-float fixed z-[1035]" :style="panelStyle">
    <button
      v-if="isCollapsed"
      type="button"
      class="timer-collapsed-chip"
      :title="t('studyTimer.expand')"
      data-testid="timer-collapsed-chip"
      @click="expandPanel"
    >
      {{ timerDisplay }}
    </button>

    <div
      v-else
      class="timer-expanded-panel"
      :class="{ 'is-dragging': isDragging }"
      data-testid="timer-expanded-row"
      @pointerdown="onDragStart"
    >
      <div class="timer-time-row">
        <span class="timer-display">{{ timerDisplay }}</span>
      </div>

      <div class="timer-actions" @pointerdown.stop>
        <a-button
          type="primary"
          size="small"
          class="timer-main-btn"
          :title="timerMainActionLabel"
          :aria-label="timerMainActionLabel"
          data-testid="timer-main-button"
          @click="triggerMainAction"
        >
          <template #icon>
            <PauseCircleOutlined v-if="timerStatus === 'running'" />
            <PlayCircleOutlined v-else />
          </template>
        </a-button>

        <a-button
          size="small"
          class="timer-icon-btn"
          :disabled="!hasTimerSession"
          :title="t('studyTimer.restart')"
          data-testid="timer-reset-button"
          @click="emit('timerRestart')"
        >
          <template #icon><RedoOutlined /></template>
        </a-button>

        <a-button
          size="small"
          class="timer-icon-btn"
          :disabled="!hasTimerSession"
          :title="t('studyTimer.stopAndSave')"
          data-testid="timer-save-button"
          @click="emit('timerStopSave')"
        >
          <template #icon><SaveOutlined /></template>
        </a-button>

        <a-button
          size="small"
          class="timer-icon-btn"
          :title="t('studyTimer.collapse')"
          data-testid="timer-collapse-button"
          @click="collapsePanel"
        >
          <template #icon><MenuFoldOutlined /></template>
        </a-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.study-timer-float {
  pointer-events: auto;
}

.timer-expanded-panel {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  min-width: 230px;
  border: 1px solid color-mix(in srgb, var(--timer-border) 74%, transparent);
  border-radius: 16px;
  background: linear-gradient(
    145deg,
    color-mix(in srgb, var(--timer-bg) 80%, #ffffff 20%),
    color-mix(in srgb, var(--timer-bg) 96%, #000000 4%)
  );
  box-shadow:
    0 10px 26px rgb(15 23 42 / 18%),
    inset 0 1px 0 rgb(255 255 255 / 32%);
  backdrop-filter: blur(16px);
  color: var(--timer-text);
  cursor: grab;
  user-select: none;
  padding: 10px;
}

.timer-expanded-panel.is-dragging {
  cursor: grabbing;
}

.timer-time-row {
  display: flex;
  justify-content: center;
}

.timer-display {
  width: 100%;
  border: 1px solid color-mix(in srgb, var(--timer-border) 72%, transparent);
  border-radius: 12px;
  background: color-mix(in srgb, var(--timer-bg) 76%, #ffffff 24%);
  padding: 8px 10px;
  text-align: center;
  font-size: 21px;
  font-weight: 800;
  letter-spacing: 0.02em;
  font-variant-numeric: tabular-nums;
  line-height: 1;
}

.timer-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
}

:deep(.timer-main-btn.ant-btn) {
  width: 36px;
  min-width: 36px;
  height: 36px;
  border-radius: 10px;
  padding-inline: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px color-mix(in srgb, var(--timer-border) 26%, transparent);
}

:deep(.timer-main-btn.ant-btn .ant-btn-icon) {
  width: 100%;
  height: 100%;
  margin: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

:deep(.timer-icon-btn.ant-btn) {
  width: 36px;
  min-width: 36px;
  height: 36px;
  padding-inline: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  border: 1px solid color-mix(in srgb, var(--timer-border) 70%, transparent);
  background: color-mix(in srgb, var(--timer-bg) 86%, #ffffff 14%);
}

:deep(.timer-icon-btn.ant-btn .ant-btn-icon) {
  width: 100%;
  height: 100%;
  margin: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

:deep(.timer-main-btn.ant-btn .anticon),
:deep(.timer-icon-btn.ant-btn .anticon) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
  vertical-align: 0;
}

:deep(.timer-main-btn.ant-btn .anticon) {
  font-size: 16px;
}

:deep(.timer-icon-btn.ant-btn .anticon) {
  font-size: 15px;
}

:deep(.timer-icon-btn.ant-btn:not(:disabled):hover) {
  border-color: color-mix(in srgb, var(--timer-border) 90%, transparent);
  background: color-mix(in srgb, var(--timer-bg) 74%, #ffffff 26%);
}

:deep(.timer-icon-btn.ant-btn:focus-visible),
:deep(.timer-main-btn.ant-btn:focus-visible) {
  outline: 2px solid color-mix(in srgb, var(--timer-border) 45%, #1677ff 55%);
  outline-offset: 1px;
}

.timer-collapsed-chip {
  border: 1px solid color-mix(in srgb, var(--timer-border) 72%, transparent);
  border-radius: 12px 0 0 12px;
  background: linear-gradient(
    145deg,
    color-mix(in srgb, var(--timer-bg) 86%, #ffffff 14%),
    color-mix(in srgb, var(--timer-bg) 96%, #000000 4%)
  );
  box-shadow: 0 10px 24px rgb(15 23 42 / 17%);
  backdrop-filter: blur(16px);
  color: var(--timer-text);
  font-size: 14px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition:
    background-color 0.2s ease,
    border-color 0.2s ease;
  padding: 8px 12px;
  min-width: 104px;
}

.timer-collapsed-chip:hover {
  border-color: color-mix(in srgb, var(--timer-border) 92%, transparent);
  background: color-mix(in srgb, var(--timer-bg) 76%, #ffffff 24%);
}
</style>
