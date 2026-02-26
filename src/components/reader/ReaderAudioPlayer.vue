<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useReaderStore } from '../../stores/reader'
import { useReaderAudio } from '../../composables/useReaderAudio'
import {
  PlayCircleFilled,
  PauseCircleFilled,
  CloseOutlined,
  StepBackwardOutlined,
  StepForwardOutlined,
  HolderOutlined,
  MinusOutlined,
  PlusOutlined,
} from '@ant-design/icons-vue'
import { theme } from 'ant-design-vue'

const { useToken } = theme
const { token } = useToken()

const readerStore = useReaderStore()
const { isPlaying, audioCurrentTime, audioDuration, currentAudioPath } = storeToRefs(readerStore)

const {
  seekAudio,
  pauseAudio,
  togglePlay: toggleAudioPlay,
  handleAudioSliderChange,
} = useReaderAudio()

const isCollapsed = ref(false)
const isDragging = ref(false)
const isSeeking = ref(false)
const progressRef = ref<HTMLElement | null>(null)
const startPos = ref({ x: 0, y: 0 })

const COLLAPSED_WIDTH = 56
const EXPANDED_WIDTH = 240

// Use state for position to survive re-renders
const position = ref({ x: window.innerWidth - EXPANDED_WIDTH - 20, y: 80 })
const lastExpandedPosition = ref({ x: position.value.x, y: position.value.y })

// Handle position memory and auto-snap
watch(isCollapsed, (val) => {
  if (val) {
    lastExpandedPosition.value = { ...position.value }
    position.value.x = window.innerWidth - COLLAPSED_WIDTH
  } else {
    position.value = { ...lastExpandedPosition.value }
  }
})

const formatTime = (seconds: number) => {
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

const audioProgress = computed(() => {
  if (audioDuration.value === 0) return 0
  return (audioCurrentTime.value / audioDuration.value) * 100
})

// Drag Player logic
const onDragStart = (e: PointerEvent) => {
  isDragging.value = true
  startPos.value = { x: e.clientX - position.value.x, y: e.clientY - position.value.y }

  const el = e.currentTarget as HTMLElement
  el.setPointerCapture(e.pointerId)

  el.addEventListener('pointermove', onDragMove as any)
  el.addEventListener('pointerup', onDragEnd as any)
  el.addEventListener('pointercancel', onDragEnd as any)
}

const onDragMove = (e: PointerEvent) => {
  if (!isDragging.value) return
  let newX = e.clientX - startPos.value.x
  let newY = e.clientY - startPos.value.y
  const width = isCollapsed.value ? COLLAPSED_WIDTH : EXPANDED_WIDTH
  newX = Math.max(0, Math.min(window.innerWidth - width, newX))
  newY = Math.max(0, Math.min(window.innerHeight - 100, newY))
  position.value = { x: newX, y: newY }
}

const onDragEnd = (e: PointerEvent) => {
  isDragging.value = false
  const el = e.currentTarget as HTMLElement
  el.releasePointerCapture(e.pointerId)

  el.removeEventListener('pointermove', onDragMove as any)
  el.removeEventListener('pointerup', onDragEnd as any)
  el.removeEventListener('pointercancel', onDragEnd as any)
}

// Seek logic
const handleSeek = (e: PointerEvent) => {
  if (!progressRef.value || audioDuration.value === 0) return
  const rect = progressRef.value.getBoundingClientRect()
  const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left))
  const percentage = x / rect.width
  handleAudioSliderChange(percentage * audioDuration.value)
}

const onProgressMouseDown = (e: PointerEvent) => {
  isSeeking.value = true
  handleSeek(e)

  const el = e.currentTarget as HTMLElement
  el.setPointerCapture(e.pointerId)

  el.addEventListener('pointermove', onProgressMouseMove as any)
  el.addEventListener('pointerup', onProgressMouseUp as any)
  el.addEventListener('pointercancel', onProgressMouseUp as any)
}

const onProgressMouseMove = (e: PointerEvent) => {
  if (isSeeking.value) handleSeek(e)
}

const onProgressMouseUp = (e: PointerEvent) => {
  isSeeking.value = false
  const el = e.currentTarget as HTMLElement
  el.releasePointerCapture(e.pointerId)

  el.removeEventListener('pointermove', onProgressMouseMove as any)
  el.removeEventListener('pointerup', onProgressMouseUp as any)
  el.removeEventListener('pointercancel', onProgressMouseUp as any)
}

function handleTogglePlay() {
  if (currentAudioPath.value) {
    toggleAudioPlay()
  }
}

function closePlayer() {
  pauseAudio()
  readerStore.resetAudio()
}
</script>

<template>
  <Transition name="fade">
    <div
      v-if="currentAudioPath"
      class="reader-audio-player fixed transition-all duration-300"
      :class="{ 'is-dragging': isDragging, 'is-collapsed': isCollapsed }"
      :style="{
        backgroundColor: token.colorBgElevated + 'cc',
        borderColor: token.colorBorderSecondary,
        backdropFilter: 'blur(20px)',
        zIndex: 1050,
        left: `${position.x}px`,
        top: `${position.y}px`,
      }"
    >
      <!-- Header / Drag Handle -->
      <div
        v-if="!isCollapsed"
        class="drag-handle flex cursor-move items-center justify-between px-4 py-2.5"
        @pointerdown="onDragStart"
      >
        <div class="mr-2 flex flex-1 items-center gap-2 overflow-hidden">
          <HolderOutlined class="flex-shrink-0 text-xs text-gray-400" />
          <span
            class="truncate text-[10px] font-bold uppercase tabular-nums tracking-widest opacity-30"
          >
            {{ currentAudioPath.split('/').pop() }}
          </span>
        </div>
        <div class="no-drag flex items-center gap-1" @pointerdown.stop>
          <a-button type="text" size="small" class="header-btn" @click="isCollapsed = true">
            <template #icon><MinusOutlined class="text-[10px]" /></template>
          </a-button>
          <a-button
            type="text"
            size="small"
            class="header-btn hover:text-red-500"
            @click="closePlayer"
          >
            <template #icon><CloseOutlined class="text-[10px]" /></template>
          </a-button>
        </div>
      </div>

      <!-- Expanded Player -->
      <div v-if="!isCollapsed" class="player-content px-5 pb-5 pt-1">
        <!-- Main Controls -->
        <div class="mb-4 mt-1 flex items-center justify-center gap-4" @pointerdown.stop>
          <a-button type="text" shape="circle" class="control-btn" @click="seekAudio(-3)">
            <template #icon><StepBackwardOutlined style="font-size: 16px" /></template>
          </a-button>

          <button
            class="main-play-btn flex items-center justify-center shadow-md transition-all duration-300 hover:scale-105 active:scale-95"
            :style="{ backgroundColor: token.colorPrimary }"
            @click="handleTogglePlay"
          >
            <PauseCircleFilled v-if="isPlaying" style="font-size: 24px; color: white" />
            <PlayCircleFilled v-else style="font-size: 24px; color: white" />
          </button>

          <a-button type="text" shape="circle" class="control-btn" @click="seekAudio(3)">
            <template #icon><StepForwardOutlined style="font-size: 16px" /></template>
          </a-button>
        </div>

        <!-- Integrated Progress and Time Display -->
        <div class="flex items-center gap-3 px-1">
          <span class="font-mono text-[9px] font-bold tabular-nums opacity-40">{{
            formatTime(audioCurrentTime)
          }}</span>

          <div
            ref="progressRef"
            class="group/progress no-drag flex h-3 flex-1 cursor-pointer items-center"
            @pointerdown.stop="onProgressMouseDown"
          >
            <div
              class="relative h-1 w-full overflow-hidden rounded-full bg-black/5 dark:bg-white/10"
            >
              <div
                class="h-full transition-all duration-100 ease-linear"
                :class="{ 'pulse-animation': isPlaying, 'no-transition': isSeeking }"
                :style="{ width: `${audioProgress}%`, backgroundColor: token.colorPrimary }"
              ></div>
            </div>
          </div>

          <span class="font-mono text-[9px] font-bold tabular-nums opacity-40">{{
            formatTime(audioDuration)
          }}</span>
        </div>
      </div>

      <!-- Mini Mode (Collapsed) -->
      <div
        v-else
        class="mini-mode-container flex cursor-move flex-col items-center gap-2 p-2"
        @pointerdown="onDragStart"
      >
        <div class="no-drag mb-0.5 flex items-center gap-1" @pointerdown.stop>
          <a-button type="text" size="small" class="mini-action-btn" @click="isCollapsed = false">
            <template #icon><PlusOutlined class="text-[10px]" /></template>
          </a-button>
          <a-button
            type="text"
            size="small"
            class="mini-action-btn hover:text-red-500"
            @click="closePlayer"
          >
            <template #icon><CloseOutlined class="text-[10px]" /></template>
          </a-button>
        </div>

        <button
          class="flex h-10 w-10 items-center justify-center rounded-full bg-white/5 shadow-sm transition-all hover:bg-white/10 active:scale-90"
          @pointerdown.stop
          @click="handleTogglePlay"
        >
          <PauseCircleFilled
            v-if="isPlaying"
            :style="{ color: token.colorPrimary, fontSize: '24px' }"
          />
          <PlayCircleFilled v-else :style="{ color: token.colorPrimary, fontSize: '24px' }" />
        </button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.reader-audio-player {
  width: 240px;
  border-radius: 16px;
  border: 1px solid;
  user-select: none;
  box-shadow:
    0 10px 25px -5px rgba(0, 0, 0, 0.1),
    0 8px 10px -6px rgba(0, 0, 0, 0.1);
  overflow: hidden;
}

.reader-audio-player.is-collapsed {
  width: 56px;
  border-radius: 28px;
}

.is-dragging {
  box-shadow:
    0 20px 25px -5px rgba(0, 0, 0, 0.1),
    0 10px 10px -5px rgba(0, 0, 0, 0.04);
  opacity: 0.95;
  cursor: grabbing !important;
  transition: none !important;
}

.header-btn,
.mini-action-btn {
  padding: 0;
  height: 22px;
  width: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.2;
  transition: all 0.2s ease;
  border-radius: 6px;
}

.header-btn:hover,
.mini-action-btn:hover {
  opacity: 0.8;
  background-color: rgba(0, 0, 0, 0.05);
}

.dark .header-btn:hover,
.dark .mini-action-btn:hover {
  background-color: rgba(255, 255, 255, 0.1);
}

.main-play-btn {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  border: none;
  cursor: pointer;
  outline: none;
}

.control-btn {
  color: v-bind('token.colorTextSecondary');
  opacity: 0.4;
  transition: all 0.2s ease;
  height: 32px;
  width: 32px;
}

.control-btn:hover {
  opacity: 1;
  color: v-bind('token.colorPrimary');
  background-color: v-bind('token.colorFillTertiary');
}

.pulse-animation {
  animation: bar-pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes bar-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

.fade-enter-active,
.fade-leave-active {
  transition:
    opacity 0.4s cubic-bezier(0.4, 0, 0.2, 1),
    transform 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(20px) scale(0.9);
}

.no-drag {
  -webkit-app-region: no-drag;
}

/* Ensure no transition during dragging seek */
.no-transition {
  transition: none !important;
}
</style>
