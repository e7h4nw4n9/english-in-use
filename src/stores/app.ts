import { defineStore } from 'pinia'
import { ref, computed, nextTick } from 'vue'
import type { AppConfig, ConnectionStatus, Book } from '../types'
import { loadConfig, checkConnectionStatus, initializeDatabase } from '../lib/api'
import { info, error, debug } from '@tauri-apps/plugin-log'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { notification } from 'ant-design-vue'
import i18n from '../i18n'

export type GlobalLoadingToken = number

interface GlobalLoadingEntry {
  message: string
  progress: number | null
}

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
  const globalLoadingEntries = new Map<GlobalLoadingToken, GlobalLoadingEntry>()
  let nextGlobalLoadingToken = 0

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

  /** 注册一项需要全局遮罩展示的异步操作。
   * @param message - 操作对应的加载提示。
   */
  function startGlobalLoading(message?: string): GlobalLoadingToken {
    const token = ++nextGlobalLoadingToken
    const entry = {
      message: message || i18n.global.t('app.loading'),
      progress: null,
    }
    globalLoadingEntries.set(token, entry)
    globalLoading.value = true
    globalLoadingMessage.value = entry.message
    globalLoadingProgress.value = entry.progress
    return token
  }

  /** 更新指定操作的全局加载提示。
   * @param token - 开始加载时返回的操作令牌。
   * @param message - 新的加载提示。
   */
  function setGlobalLoadingMessage(token: GlobalLoadingToken, message: string) {
    const entry = globalLoadingEntries.get(token)
    if (!entry) return
    entry.message = message
    if (token !== nextActiveGlobalLoadingToken()) return
    globalLoadingMessage.value = message
  }

  /** 更新指定操作的全局加载进度。
   * @param token - 开始加载时返回的操作令牌。
   * @param progress - 新进度，传入 null 表示不展示进度。
   */
  function setGlobalLoadingProgress(token: GlobalLoadingToken, progress: number | null) {
    const entry = globalLoadingEntries.get(token)
    if (!entry) return
    entry.progress =
      progress === null || Number.isNaN(progress) ? null : Math.max(0, Math.min(100, progress))
    if (token !== nextActiveGlobalLoadingToken()) return
    globalLoadingProgress.value = entry.progress
  }

  /** 返回最后注册且仍在运行的全局加载令牌。 */
  function nextActiveGlobalLoadingToken(): GlobalLoadingToken | null {
    const tokens = Array.from(globalLoadingEntries.keys())
    return tokens.length > 0 ? tokens[tokens.length - 1] : null
  }

  /** 结束指定操作的全局加载展示。
   * @param token - 开始加载时返回的操作令牌。
   */
  function stopGlobalLoading(token: GlobalLoadingToken) {
    if (!globalLoadingEntries.delete(token)) return
    const activeToken = nextActiveGlobalLoadingToken()
    if (activeToken === null) {
      globalLoading.value = false
      globalLoadingMessage.value = ''
      globalLoadingProgress.value = null
      return
    }

    const activeEntry = globalLoadingEntries.get(activeToken)!
    globalLoadingMessage.value = activeEntry.message
    globalLoadingProgress.value = activeEntry.progress
  }

  /** 在全局加载遮罩下执行一次异步操作。
   * @param action - 需要执行的异步操作。
   * @param message - 与当前操作对应的全局加载提示。
   */
  async function runGlobalLoadingAction<T>(action: () => Promise<T>, message: string): Promise<T> {
    const token = startGlobalLoading(message)
    try {
      // 等待遮罩渲染后再发起操作，避免长任务开始后界面仍未反馈。
      await nextTick()
      return await action()
    } finally {
      stopGlobalLoading(token)
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
    runGlobalLoadingAction,
  }
})
