<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { theme } from 'ant-design-vue'
import {
  BugOutlined,
  HomeOutlined,
  CloudOutlined,
  SyncOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons-vue'
import { useAppStore } from '../stores/app'
import { useReaderStore } from '../stores/reader'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import type { ServiceStatusType } from '../types'

const { t } = useI18n()
const { useToken } = theme
const { token } = useToken()

const appStore = useAppStore()
const readerStore = useReaderStore()
const { currentBook, connectionStatus: status } = storeToRefs(appStore)
const { currentUnitName, debugVisible } = storeToRefs(readerStore)

defineProps<{
  title: string
}>()

const isFullscreen = ref(false)
const isMacOS = ref(false)
const connectionModalVisible = ref(false)

function startDrag() {
  getCurrentWindow().startDragging()
}

async function toggleMaximize() {
  await getCurrentWindow().toggleMaximize()
}

const getStatusColor = (s: ServiceStatusType) => {
  switch (s.status) {
    case 'Connected':
      return token.value.colorSuccess
    case 'Disconnected':
      return token.value.colorError
    case 'Testing':
      return token.value.colorInfo
    default:
      return token.value.colorTextDisabled
  }
}

const getStatusText = (s: ServiceStatusType) => {
  switch (s.status) {
    case 'Connected':
      return t('footer.connected')
    case 'Disconnected':
      return t('footer.disconnected')
    case 'Testing':
      return t('footer.testing')
    default:
      return t('footer.notConfigured')
  }
}

const isTesting = computed(
  () => status.value.r2.status === 'Testing' || status.value.d1.status === 'Testing',
)
const hasError = computed(
  () => status.value.r2.status === 'Disconnected' || status.value.d1.status === 'Disconnected',
)

const overallStatusColor = computed(() => {
  if (isTesting.value) return token.value.colorInfo
  if (hasError.value) return token.value.colorError
  if (status.value.r2.status === 'Connected' || status.value.d1.status === 'Connected')
    return token.value.colorSuccess
  return token.value.colorTextDisabled
})

onMounted(async () => {
  isFullscreen.value = await getCurrentWindow().isFullscreen()
  isMacOS.value = navigator.userAgent.includes('Mac')
})
</script>

<template>
  <div
    data-tauri-drag-region
    class="titlebar"
    :class="{ 'is-macos': isMacOS && !isFullscreen }"
    @mousedown="startDrag"
    @dblclick="toggleMaximize"
  >
    <div
      data-tauri-drag-region
      class="relative flex h-full w-full items-center justify-between px-3"
    >
      <!-- Left: Placeholder (for symmetry if needed) -->
      <div class="no-drag z-10 flex min-w-[80px] items-center gap-1"></div>

      <!-- Center: Title -->
      <div class="pointer-events-none absolute inset-0 flex items-center justify-center">
        <div data-tauri-drag-region class="title-text pointer-events-auto max-w-[60%] truncate">
          {{ currentBook ? currentUnitName : title }}
        </div>
      </div>

      <!-- Right: Actions (Status, Debug) -->
      <div class="no-drag z-10 flex min-w-[80px] items-center justify-end gap-1">
        <!-- Connection Status Trigger -->
        <a-tooltip placement="bottomRight" :mouse-enter-delay="0.5">
          <template #title>
            <div class="flex flex-col gap-1 py-1 text-[10px]">
              <div class="flex items-center justify-between gap-4">
                <span class="opacity-70">Cloudflare R2</span>
                <span :style="{ color: getStatusColor(status.r2) }">{{
                  getStatusText(status.r2)
                }}</span>
              </div>
              <div class="flex items-center justify-between gap-4">
                <span class="opacity-70">Cloudflare D1</span>
                <span :style="{ color: getStatusColor(status.d1) }">{{
                  getStatusText(status.d1)
                }}</span>
              </div>
            </div>
          </template>
          <a-button
            type="text"
            size="small"
            class="action-btn"
            :class="{ 'is-testing': isTesting }"
            @click.stop="connectionModalVisible = true"
          >
            <template #icon>
              <ExclamationCircleOutlined v-if="hasError" :style="{ color: overallStatusColor }" />
              <CloudOutlined v-else :style="{ color: overallStatusColor }" />
            </template>
          </a-button>
        </a-tooltip>

        <a-button
          v-if="currentBook"
          type="text"
          size="small"
          class="action-btn"
          @click.stop="debugVisible = !debugVisible"
        >
          <template #icon><BugOutlined /></template>
        </a-button>
      </div>
    </div>

    <!-- Connection Status Modal -->
    <a-modal
      v-model:open="connectionModalVisible"
      :title="t('footer.connectionStatus')"
      :footer="null"
      :width="400"
      centered
      destroy-on-close
    >
      <div class="flex flex-col gap-6 py-4">
        <!-- R2 Status -->
        <div class="status-card">
          <div class="mb-2 flex items-center justify-between">
            <span class="text-xs font-bold uppercase tracking-wider opacity-60">Cloudflare R2</span>
            <a-tag
              :color="
                status.r2.status === 'Connected'
                  ? 'success'
                  : status.r2.status === 'Disconnected'
                    ? 'error'
                    : 'default'
              "
            >
              {{ getStatusText(status.r2) }}
            </a-tag>
          </div>
          <div
            class="flex items-start gap-3 rounded-lg border border-gray-100 bg-gray-50 p-3 dark:border-white/10 dark:bg-white/5"
          >
            <div class="mt-0.5">
              <CheckCircleOutlined
                v-if="status.r2.status === 'Connected'"
                :style="{ color: token.colorSuccess }"
              />
              <SyncOutlined
                v-else-if="status.r2.status === 'Testing'"
                spin
                :style="{ color: token.colorInfo }"
              />
              <ExclamationCircleOutlined
                v-else-if="status.r2.status === 'Disconnected'"
                :style="{ color: token.colorError }"
              />
              <ClockCircleOutlined v-else :style="{ color: token.colorTextDisabled }" />
            </div>
            <div class="flex-1 overflow-hidden">
              <div class="text-xs font-medium">
                {{
                  status.r2.status === 'Disconnected' ? status.r2.message : getStatusText(status.r2)
                }}
              </div>
            </div>
          </div>
        </div>

        <!-- D1 Status -->
        <div class="status-card">
          <div class="mb-2 flex items-center justify-between">
            <span class="text-xs font-bold uppercase tracking-wider opacity-60">Cloudflare D1</span>
            <a-tag
              :color="
                status.d1.status === 'Connected'
                  ? 'success'
                  : status.d1.status === 'Disconnected'
                    ? 'error'
                    : 'default'
              "
            >
              {{ getStatusText(status.d1) }}
            </a-tag>
          </div>
          <div
            class="flex items-start gap-3 rounded-lg border border-gray-100 bg-gray-50 p-3 dark:border-white/10 dark:bg-white/5"
          >
            <div class="mt-0.5">
              <CheckCircleOutlined
                v-if="status.d1.status === 'Connected'"
                :style="{ color: token.colorSuccess }"
              />
              <SyncOutlined
                v-else-if="status.d1.status === 'Testing'"
                spin
                :style="{ color: token.colorInfo }"
              />
              <ExclamationCircleOutlined
                v-else-if="status.d1.status === 'Disconnected'"
                :style="{ color: token.colorError }"
              />
              <ClockCircleOutlined v-else :style="{ color: token.colorTextDisabled }" />
            </div>
            <div class="flex-1 overflow-hidden">
              <div class="text-xs font-medium">
                {{
                  status.d1.status === 'Disconnected' ? status.d1.message : getStatusText(status.d1)
                }}
              </div>
            </div>
          </div>
        </div>

        <div class="mt-4 flex justify-end border-t border-gray-100 pt-4 dark:border-white/10">
          <a-button type="primary" :loading="isTesting" @click="appStore.updateConnectionStatus">
            <template #icon><SyncOutlined /></template>
            {{ t('footer.recheck') }}
          </a-button>
        </div>
      </div>
    </a-modal>
  </div>
</template>

<style scoped>
.titlebar {
  height: 32px;
  user-select: none;
  display: flex;
  align-items: center;
  width: 100%;
  flex-shrink: 0;
  z-index: 1000;
  cursor: default;
  background-color: v-bind('token.colorBgContainer');
  border-bottom: 1px solid v-bind('token.colorBorderSecondary');
  position: relative;
  overflow: hidden;
  transition: padding-left 0.3s ease;
}

.is-macos {
  padding-left: 72px;
}

.titlebar::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, v-bind('token.colorPrimary'), transparent);
  opacity: 0.1;
}

.title-content {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}

.title-text {
  font-weight: 700;
  font-size: 14px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: v-bind('token.colorTextSecondary');
}

.no-drag {
  -webkit-app-region: no-drag;
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  color: v-bind('token.colorTextSecondary');
  opacity: 0.6;
}

.action-btn:hover {
  opacity: 1;
  background-color: v-bind('token.colorFillTertiary');
}

.is-testing {
  opacity: 1;
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}

.status-card {
  display: flex;
  flex-direction: column;
}
</style>
