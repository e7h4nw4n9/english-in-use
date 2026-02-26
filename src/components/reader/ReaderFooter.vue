<script setup lang="ts">
import { ref, computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '../../stores/app'
import { useReaderStore } from '../../stores/reader'
import {
  LeftOutlined,
  RightOutlined,
  UnorderedListOutlined,
  EyeOutlined,
  EyeInvisibleOutlined,
  FullscreenExitOutlined,
  FileTextOutlined,
  BlockOutlined,
  HomeOutlined,
  AppstoreOutlined,
} from '@ant-design/icons-vue'
import { theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import type { OverlayAudio, ExerciseInfo } from '../../types'

const { useToken } = theme
const { token } = useToken()
const { t } = useI18n()

const props = defineProps<{
  displayIndex: number
  sortedPageLabels: string[]
  currentPageAudioFiles: OverlayAudio[]
  currentPageExercises: ExerciseInfo[]
  isNarrow?: boolean
}>()

const emit = defineEmits<{
  (e: 'toggleAudio', path: string): void
  (e: 'openExercise', ex: ExerciseInfo): void
  (e: 'goBack'): void
  (e: 'goForward'): void
}>()

const appStore = useAppStore()
const readerStore = useReaderStore()
const { viewMode, showHotspots, resourceDrawerVisible, isSidebarCollapsed } =
  storeToRefs(readerStore)

const drawerTab = ref('audio')

const currentRangeText = computed(() => {
  const left = props.sortedPageLabels[props.displayIndex] || ''
  if (viewMode.value === 'single') {
    return left
  }
  const right = props.sortedPageLabels[props.displayIndex + 1] || ''
  if (!right) return left
  return `${left}-${right}`
})

function closeReader() {
  appStore.currentBook = null
}

function toggleSidebar() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value
}
</script>

<template>
  <div class="reader-footer-floating">
    <!-- 1. Integrated Page Indicator & Navigation (Bottom Center) -->
    <div
      class="pointer-events-auto fixed bottom-6 left-1/2 z-[1000] flex -translate-x-1/2 items-center gap-3 rounded-full border px-3 py-1.5 shadow-lg backdrop-blur-md transition-all duration-300"
      :style="{
        backgroundColor: token.colorBgElevated + 'aa',
        borderColor: token.colorBorderSecondary,
        color: token.colorTextSecondary,
      }"
    >
      <!-- Prev Button -->
      <a-button
        type="text"
        size="small"
        class="flex h-auto items-center p-0 text-inherit opacity-60 transition-opacity hover:opacity-100"
        :title="t('reader.prevPage')"
        @click="emit('goBack')"
      >
        <template #icon><LeftOutlined /></template>
      </a-button>

      <!-- Page Range Text -->
      <div class="min-w-[60px] px-1 text-center text-xs font-bold tabular-nums tracking-wider">
        {{ currentRangeText }} <span class="mx-0.5 opacity-40">/</span>
        {{ sortedPageLabels.length }}
      </div>

      <!-- View Mode Toggle -->
      <a-button
        type="text"
        size="small"
        class="flex h-auto items-center p-0 text-inherit opacity-60 transition-opacity hover:opacity-100"
        :disabled="isNarrow"
        :title="viewMode === 'single' ? t('reader.viewSpread') : t('reader.viewSingle')"
        @click="viewMode = viewMode === 'single' ? 'spread' : 'single'"
      >
        <template #icon>
          <BlockOutlined v-if="viewMode === 'single'" />
          <FileTextOutlined v-else />
        </template>
      </a-button>

      <!-- Next Button -->
      <a-button
        type="text"
        size="small"
        class="flex h-auto items-center p-0 text-inherit opacity-60 transition-opacity hover:opacity-100"
        :title="t('reader.nextPage')"
        @click="emit('goForward')"
      >
        <template #icon><RightOutlined /></template>
      </a-button>
    </div>

    <!-- 2. Left Side Action Buttons -->
    <a-float-button
      type="primary"
      :style="{ left: '24px', bottom: '24px' }"
      class="soft-primary-btn"
      @click="closeReader"
    >
      <template #icon><HomeOutlined /></template>
      <template #tooltip>{{ t('reader.home') }}</template>
    </a-float-button>

    <a-float-button
      type="primary"
      :style="{ left: '24px', bottom: '80px' }"
      class="soft-primary-btn"
      @click="toggleSidebar"
    >
      <template #icon><UnorderedListOutlined /></template>
      <template #tooltip>{{ t('reader.toc') }}</template>
    </a-float-button>

    <!-- 3. Function Island (Bottom Right) -->
    <a-float-button-group
      trigger="click"
      type="primary"
      :style="{ right: '24px', bottom: '24px' }"
      class="soft-primary-btn"
    >
      <template #icon><AppstoreOutlined /></template>

      <!-- Hotspots Toggle -->
      <a-float-button @click="showHotspots = !showHotspots" type="primary" class="soft-primary-btn">
        <template #icon>
          <EyeOutlined v-if="showHotspots" />
          <EyeInvisibleOutlined v-else />
        </template>
        <template #tooltip>{{
          showHotspots ? t('reader.hideHotspots') : t('reader.showHotspots')
        }}</template>
      </a-float-button>

      <!-- Zoom Reset -->
      <a-float-button @click="readerStore.resetZoom()" type="primary" class="soft-primary-btn">
        <template #icon><FullscreenExitOutlined /></template>
        <template #tooltip>{{ t('reader.resetZoom') }}</template>
      </a-float-button>
    </a-float-button-group>

    <!-- Resources Drawer removed as per functionality cleanup -->
  </div>
</template>

<style scoped>
.reader-footer-floating {
  pointer-events: none;
}
.reader-footer-floating :deep(.ant-float-btn),
.reader-footer-floating :deep(.ant-btn),
.reader-footer-floating :deep(.ant-drawer) {
  pointer-events: auto;
}

.nav-btn {
  color: v-bind('token.colorTextSecondary');
}
.nav-btn:hover {
  color: v-bind('token.colorPrimary');
}

/* Soft Primary Button Styles */
:deep(.soft-primary-btn.ant-float-btn-primary .ant-float-btn-body) {
  background-color: v-bind('token.colorPrimaryBg') !important;
}
:deep(.soft-primary-btn.ant-float-btn-primary .ant-float-btn-icon) {
  color: v-bind('token.colorPrimary') !important;
  opacity: 0.65;
}
:deep(.soft-primary-btn.ant-float-btn-primary:hover .ant-float-btn-body) {
  background-color: v-bind('token.colorPrimaryBgHover') !important;
}
</style>
