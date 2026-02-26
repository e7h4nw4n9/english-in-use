<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useReaderStore } from '../../stores/reader'
import type { BookMetadata } from '../../types'
import {
  LeftOutlined,
  RightOutlined,
  CustomerServiceOutlined,
  LinkOutlined,
} from '@ant-design/icons-vue'

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

const readerStore = useReaderStore()
const { viewMode, zoomLevel } = storeToRefs(readerStore)

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

const pageSurfaceStyle = computed(() => {
  if (!props.metadata) return {}
  const ratio = props.metadata.pageWidth / props.metadata.pageHeight
  return {
    aspectRatio: `${ratio}`,
    width: viewMode.value === 'spread' ? '0' : '100%',
    flex: viewMode.value === 'spread' ? '1 1 0' : 'none',
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
    <div
      v-if="loading"
      class="absolute inset-0 z-20 flex items-center justify-center bg-white/50 dark:bg-black/50"
    >
      <a-spin size="large" tip="正在加载内容..." />
    </div>

    <div class="custom-scrollbar flex flex-1 justify-center overflow-auto" @wheel="handleWheel">
      <div
        class="reader-content-container flex w-full origin-top items-start justify-center gap-4 px-0 py-1 transition-transform duration-200"
        :style="{ transform: `scale(${zoomLevel})` }"
      >
        <!-- Left Page -->
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
              <div class="icon-wrapper flex h-9 w-9 items-center justify-center rounded-full">
                <CustomerServiceOutlined v-if="overlay.type === 'audio'" />
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

        <!-- Placeholder for Empty Left Page in Spread Mode -->
        <div
          v-else-if="viewMode === 'spread'"
          :style="pageSurfaceStyle"
          class="pointer-events-none opacity-0"
        ></div>

        <!-- Right Page -->
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
              <div class="icon-wrapper flex h-9 w-9 items-center justify-center rounded-full">
                <CustomerServiceOutlined v-if="overlay.type === 'audio'" />
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

.dark .icon-wrapper {
  background: rgba(255, 255, 255, 0.8);
  border-color: rgba(255, 255, 255, 0.2);
}

/* 统一深色图标 */
.overlay-item .anticon {
  @apply transition-transform duration-300;
  font-size: 20px !important;
  color: #1e293b; /* 优雅的深灰色 */
  filter: drop-shadow(0 1px 1px rgba(255, 255, 255, 0.5));
}

.overlay-item:hover .icon-wrapper {
  @apply scale-110 shadow-md;
  background: rgba(255, 255, 255, 0.9);
}

.overlay-item:hover {
  @apply z-30;
}
</style>
