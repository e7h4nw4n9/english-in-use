import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { AppConfig, ConnectionStatus, Book } from '../types'
import { loadConfig, checkConnectionStatus, initializeDatabase } from '../lib/api'
import { info, error, debug } from '@tauri-apps/plugin-log'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { notification } from 'ant-design-vue'
import i18n from '../i18n'

export const useAppStore = defineStore('app', () => {
  const config = ref<AppConfig | null>(null)
  const connectionStatus = ref<ConnectionStatus>({
    r2: { status: 'NotConfigured' },
    d1: { status: 'NotConfigured' },
  })
  const isLoading = ref(true)
  const loadingMessage = ref('')
  const currentBook = ref<Book | null>(null)

  let unlistenStatus: UnlistenFn | null = null

  function sanitizeErrorMessage(message: string): string {
    return message.replace(/https?:\/\/[^\s]+/g, '[URL]')
  }

  const isConfigValid = computed(() => {
    if (!config.value) return false

    // Check book source
    const bs = config.value.book_source
    if (!bs) return false
    if (bs.type === 'Local') {
      if (!bs.details.path) return false
    } else if (bs.type === 'CloudflareR2') {
      const d = bs.details
      if (!d.account_id || !d.bucket_name || !d.access_key_id || !d.secret_access_key) return false
    }

    // Check database
    const db = config.value.database
    if (!db) return false
    if (db.type === 'SQLite') {
      if (!db.details.path) return false
    } else if (db.type === 'CloudflareD1') {
      const d = db.details
      if (!d.account_id || !d.database_id || !d.api_token) return false
    }

    return true
  })

  async function initApp() {
    isLoading.value = true
    loadingMessage.value = i18n.global.t('app.loading')
    info('正在初始化应用 Store...')

    if (!unlistenStatus) {
      unlistenStatus = await listen<ConnectionStatus>('connection-status-update', (event) => {
        const newStatus = event.payload
        const oldStatus = connectionStatus.value

        // Error notifications
        if (newStatus.r2.status === 'Disconnected' && oldStatus.r2.status !== 'Disconnected') {
          notification.error({
            message: i18n.global.t('footer.connectionError'),
            description: `R2: ${sanitizeErrorMessage(newStatus.r2.message)}`,
            placement: 'bottomRight',
          })
        }

        if (newStatus.d1.status === 'Disconnected' && oldStatus.d1.status !== 'Disconnected') {
          notification.error({
            message: i18n.global.t('footer.connectionError'),
            description: `D1: ${sanitizeErrorMessage(newStatus.d1.message)}`,
            placement: 'bottomRight',
          })
        }

        connectionStatus.value = newStatus
      })
    }

    try {
      const loadedConfig = await loadConfig()
      config.value = loadedConfig
      debug('应用配置已加载到 Store')

      if (isConfigValid.value) {
        info('配置有效，正在初始化数据库...')
        const newlyInitialized = await initializeDatabase()
        info(`数据库初始化完成，是否为新初始化: ${newlyInitialized}`)

        // If using Cloudflare D1 and it was newly initialized, wait a couple of seconds
        // for the database to be fully ready on the network
        if (newlyInitialized && config.value?.database?.type === 'CloudflareD1') {
          debug('检测到 Cloudflare D1 首次初始化，等待 2 秒以确保连接就绪...')
          loadingMessage.value = i18n.global.t('config.waitingForDatabase')
          await new Promise((resolve) => setTimeout(resolve, 2000))
        }

        // Initial status check
        if (config.value.system.enable_auto_check) {
          updateConnectionStatus()
        }
      }
    } catch (err) {
      error(`应用初始化失败: ${err}`)
    } finally {
      isLoading.value = false
    }
  }

  async function updateConnectionStatus() {
    try {
      const status = await checkConnectionStatus()
      connectionStatus.value = status
    } catch (err) {
      error(`获取连接状态失败: ${err}`)
    }
  }

  async function refreshConfig() {
    try {
      config.value = await loadConfig()
    } catch (err) {
      error(`刷新配置失败: ${err}`)
    }
  }

  return {
    config,
    connectionStatus,
    isLoading,
    loadingMessage,
    isConfigValid,
    currentBook,
    initApp,
    updateConnectionStatus,
    refreshConfig,
  }
})
