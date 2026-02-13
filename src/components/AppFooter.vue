<script setup lang="ts">
import { onMounted, onUnmounted, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { theme, notification } from 'ant-design-vue'
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  SyncOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons-vue'
import { useI18n } from 'vue-i18n'
import type { ConnectionStatus, ServiceStatusType } from '../types'
import { useAppStore } from '../stores/app'
import { storeToRefs } from 'pinia'

const { t } = useI18n()
const { useToken } = theme
const { token } = useToken()

const appStore = useAppStore()
const { connectionStatus: status } = storeToRefs(appStore)

let unlistenStatus: UnlistenFn | null = null

const hasR2 = computed(() => status.value.r2.status !== 'NotConfigured')
const hasD1 = computed(() => status.value.d1.status !== 'NotConfigured')
const hasActiveConfig = computed(() => hasR2.value || hasD1.value)

function sanitizeErrorMessage(message: string): string {
  // Simple regex to remove http/https URLs
  return message.replace(/https?:\/\/[^\s]+/g, '[URL]')
}

function updateStatus(newStatus: ConnectionStatus) {
  if (newStatus.r2.status === 'Disconnected' && status.value.r2.status !== 'Disconnected') {
    notification.error({
      message: t('footer.connectionError'),
      description: `R2: ${sanitizeErrorMessage(newStatus.r2.message)}`,
      placement: 'bottomRight',
    })
  }

  if (newStatus.d1.status === 'Disconnected' && status.value.d1.status !== 'Disconnected') {
    notification.error({
      message: t('footer.connectionError'),
      description: `D1: ${sanitizeErrorMessage(newStatus.d1.message)}`,
      placement: 'bottomRight',
    })
  }

  status.value = newStatus
}

onMounted(async () => {
  unlistenStatus = await listen<ConnectionStatus>('connection-status-update', (event) => {
    updateStatus(event.payload)
  })
})

onUnmounted(() => {
  if (unlistenStatus) unlistenStatus()
})

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
</script>

<template>
  <footer
    v-if="hasActiveConfig"
    class="app-footer"
    :style="{
      backgroundColor: token.colorBgContainer,
      borderTop: `1px solid ${token.colorBorderSecondary}`,
      color: token.colorTextSecondary,
    }"
  >
    <div class="status-items">
      <div
        v-if="hasR2"
        class="status-item"
        :title="status.r2.status === 'Disconnected' ? status.r2.message : ''"
      >
        <span class="label">{{ t('footer.r2Status') }}:</span>
        <span class="value" :style="{ color: getStatusColor(status.r2) }">
          <CheckCircleOutlined v-if="status.r2.status === 'Connected'" />
          <CloseCircleOutlined v-else-if="status.r2.status === 'Disconnected'" />
          <SyncOutlined v-else-if="status.r2.status === 'Testing'" spin />
          <ExclamationCircleOutlined v-else />
          <span class="text">{{ getStatusText(status.r2) }}</span>
        </span>
      </div>

      <a-divider v-if="hasR2 && hasD1" type="vertical" />

      <div
        v-if="hasD1"
        class="status-item"
        :title="status.d1.status === 'Disconnected' ? status.d1.message : ''"
      >
        <span class="label">{{ t('footer.d1Status') }}:</span>
        <span class="value" :style="{ color: getStatusColor(status.d1) }">
          <CheckCircleOutlined v-if="status.d1.status === 'Connected'" />
          <CloseCircleOutlined v-else-if="status.d1.status === 'Disconnected'" />
          <SyncOutlined v-else-if="status.d1.status === 'Testing'" spin />
          <ExclamationCircleOutlined v-else />
          <span class="text">{{ getStatusText(status.d1) }}</span>
        </span>
      </div>
    </div>

    <div class="actions">
      <a-button type="link" size="small" @click="appStore.updateConnectionStatus">
        <template #icon
          ><SyncOutlined :spin="status.r2.status === 'Testing' || status.d1.status === 'Testing'"
        /></template>
        {{ t('footer.recheck') }}
      </a-button>
    </div>
  </footer>
</template>

<style scoped>
.app-footer {
  height: 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 12px;
  font-size: 11px;
  width: 100%;
  flex-shrink: 0;
  z-index: 1000;
  user-select: none;
}

.status-items {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.label {
  font-weight: 500;
}

.value {
  display: flex;
  align-items: center;
  gap: 4px;
}

.text {
  margin-left: 2px;
}

.actions {
  display: flex;
  align-items: center;
}

:deep(.ant-btn-link) {
  font-size: 11px;
  height: 20px;
  padding: 0 4px;
  line-height: 20px;
}
</style>
