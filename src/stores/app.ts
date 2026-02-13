import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { AppConfig, ConnectionStatus } from '../types'
import { loadConfig, saveConfig, checkConnectionStatus, initializeDatabase } from '../lib/api'
import { info, error, debug } from '@tauri-apps/plugin-log'
import i18n from '../i18n'

export const useAppStore = defineStore('app', () => {
  const config = ref<AppConfig | null>(null)
  const connectionStatus = ref<ConnectionStatus>({
    r2: { status: 'NotConfigured' },
    d1: { status: 'NotConfigured' },
  })
  const isLoading = ref(true)
  const loadingMessage = ref('')

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

    try {
      const loadedConfig = await loadConfig()
      config.value = loadedConfig
      debug('应用配置已加载到 Store')

      if (isConfigValid.value) {
        info('配置有效，正在初始化数据库...')
        await initializeDatabase()
        info('数据库初始化成功')

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
    initApp,
    updateConnectionStatus,
    refreshConfig,
  }
})
