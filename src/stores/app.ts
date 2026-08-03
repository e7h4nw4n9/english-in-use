import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { AppConfig, ConnectionStatus, Book } from '../types'
import { loadConfig, checkConnectionStatus, initializeDatabase } from '../lib/api'
import { info, error, debug } from '@tauri-apps/plugin-log'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { notification } from 'ant-design-vue'
import i18n from '../i18n'

/** 应用配置、连接状态和全局加载状态存储。 */
export const useAppStore = defineStore('app', () => {
  const config = ref<AppConfig | null>(null)
  const connectionStatus = ref<ConnectionStatus>({
    r2: { status: 'NotConfigured' },
    database: { status: 'NotConfigured' },
  })
  const isLoading = ref(true)
  const loadingMessage = ref('')
  const globalLoading = ref(false)
  const globalLoadingMessage = ref('')
  const globalLoadingProgress = ref<number | null>(null)
  const currentBook = ref<Book | null>(null)
  let globalLoadingCount = 0

  let unlistenStatus: UnlistenFn | null = null

  function sanitizeErrorMessage(message: string): string {
    return message.replace(/https?:\/\/[^\s]+/g, '[URL]')
  }

  const isConfigValid = computed(() => {
    if (!config.value) return false

    // 检查图书来源连接。
    const bs = config.value.book_source
    if (!bs) return false
    if (bs.type === 'Local') {
      if (!bs.details.path) return false
    }

    // 检查数据库连接。
    const db = config.value.database
    if (!db) return false
    if (db.type === 'SQLite') {
      if (!db.details.path) return false
    }

    const usesGateway = bs.type === 'CloudflareGateway' || db.type === 'CloudflareGateway'
    if (usesGateway) {
      const gateway = config.value.cloudflare_gateway
      if (
        config.value.gateway_configuration_required ||
        !gateway?.base_url ||
        !gateway.access_token
      ) {
        return false
      }
    }

    return true
  })

  /** 加载配置、初始化数据库并启动连接监控。 */
  async function initApp() {
    isLoading.value = true
    loadingMessage.value = i18n.global.t('app.loading')
    info('正在初始化应用 Store...')

    if (!unlistenStatus) {
      unlistenStatus = await listen<ConnectionStatus>('connection-status-update', (event) => {
        const newStatus = event.payload
        const oldStatus = connectionStatus.value

        // 显示初始化失败通知。
        if (newStatus.r2.status === 'Disconnected' && oldStatus.r2.status !== 'Disconnected') {
          notification.error({
            message: i18n.global.t('footer.connectionError'),
            description: `R2: ${sanitizeErrorMessage(newStatus.r2.message)}`,
            placement: 'bottomRight',
          })
        }

        if (
          newStatus.database.status === 'Disconnected' &&
          oldStatus.database.status !== 'Disconnected'
        ) {
          notification.error({
            message: i18n.global.t('footer.connectionError'),
            description: `Database: ${sanitizeErrorMessage(newStatus.database.message)}`,
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
        loadingMessage.value = i18n.global.t('db.init.checkLatestVersion')
        const didMigrate = await initializeDatabase()
        info(`数据库初始化完成，是否执行迁移: ${didMigrate}`)

        // 执行首次连接状态检查。
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

  /** 主动刷新一次远程服务和数据库连接状态。 */
  async function updateConnectionStatus() {
    try {
      const status = await checkConnectionStatus()
      connectionStatus.value = status
    } catch (err) {
      error(`获取连接状态失败: ${err}`)
    }
  }

  /** 从后端重新加载配置并替换当前状态。 */
  async function refreshConfig() {
    try {
      config.value = await loadConfig()
    } catch (err) {
      error(`刷新配置失败: ${err}`)
    }
  }

  function startGlobalLoading(message?: string) {
    globalLoadingCount += 1
    globalLoading.value = true
    globalLoadingMessage.value = message || i18n.global.t('app.loading')
    globalLoadingProgress.value = null
  }

  function setGlobalLoadingMessage(message: string) {
    globalLoadingMessage.value = message
  }

  function setGlobalLoadingProgress(progress: number | null) {
    if (progress === null || Number.isNaN(progress)) {
      globalLoadingProgress.value = null
      return
    }
    globalLoadingProgress.value = Math.max(0, Math.min(100, progress))
  }

  function stopGlobalLoading() {
    globalLoadingCount = Math.max(0, globalLoadingCount - 1)
    if (globalLoadingCount === 0) {
      globalLoading.value = false
      globalLoadingMessage.value = ''
      globalLoadingProgress.value = null
    }
  }

  return {
    config,
    connectionStatus,
    isLoading,
    loadingMessage,
    globalLoading,
    globalLoadingMessage,
    globalLoadingProgress,
    isConfigValid,
    currentBook,
    initApp,
    updateConnectionStatus,
    refreshConfig,
    startGlobalLoading,
    setGlobalLoadingMessage,
    setGlobalLoadingProgress,
    stopGlobalLoading,
  }
})
