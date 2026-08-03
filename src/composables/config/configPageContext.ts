import { computed, reactive, ref } from 'vue'
import { message as antMessage, theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import type {
  AppConfig,
  BookSource,
  BookSourceType,
  DatabaseConnection,
  DatabaseType,
} from '../../types'
import { useTheme } from '../useTheme'
import { useAppStore } from '../../stores/app'

export type ConfigPageEmit = {
  (event: 'config-saved', config: AppConfig): void
  (event: 'config-imported'): void
  (event: 'back', options?: { reloadHome?: boolean }): void
}

/**
 * 创建配置页面共享的响应式表单状态和诊断状态。
 * @param initialConfig - 页面首次加载的配置。
 * @param allowBack - 是否显示返回入口。
 * @param emit - 配置页面事件发送器。
 */
export function createConfigPageContext(
  initialConfig: AppConfig | undefined,
  allowBack: boolean,
  emit: ConfigPageEmit,
) {
  const { t, locale } = useI18n()
  const { setTheme } = useTheme()
  const appStore = useAppStore()
  const { useToken } = theme
  const { token } = useToken()

  antMessage.config({ top: '40px', duration: 3, maxCount: 3 })

  const activeTab = ref<string[]>(['system'])

  const currentTab = computed(() => activeTab.value[0])
  const debugFeaturesAvailable = __DEBUG_FEATURES__
  const showBackButton = computed(() => allowBack !== false)

  // 系统配置。
  const language = ref(initialConfig?.system?.language || 'en')
  const themeMode = ref(initialConfig?.system?.theme || 'system')
  const logLevel = ref(initialConfig?.system?.log_level || 'info')
  const enableDebugTools = ref(initialConfig?.system?.enable_debug_tools ?? false)
  const autoStartStudyTimer = ref(initialConfig?.system?.auto_start_study_timer ?? false)
  const isCloudConfigured = computed(
    () => sourceType.value === 'CloudflareGateway' || dbType.value === 'CloudflareGateway',
  )
  const enableAutoCheck = ref(initialConfig?.system?.enable_auto_check ?? true)
  const checkIntervalMins = ref(initialConfig?.system?.check_interval_mins || 5)

  // 图书来源配置。
  const sourceType = ref<BookSourceType>(initialConfig?.book_source?.type || 'Local')
  const localBookPath = ref(
    initialConfig?.book_source?.type === 'Local' ? initialConfig.book_source.details.path : '',
  )
  const gatewayConfig = reactive({
    base_url: initialConfig?.cloudflare_gateway?.base_url || '',
    access_token: initialConfig?.cloudflare_gateway?.access_token || '',
  })
  const gatewayConfigurationRequired = ref(initialConfig?.gateway_configuration_required ?? false)

  // 数据库配置。
  const dbType = ref<DatabaseType>(initialConfig?.database?.type || 'SQLite')
  const sqlitePath = ref('')

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text)
      antMessage.success(t('common.copied' as any) || 'Copied to clipboard')
    } catch (err) {
      antMessage.error('Failed to copy')
    }
  }

  const isTesting = ref(false)
  const isExporting = ref(false)
  const isImporting = ref(false)
  const isSaving = ref(false)
  const shouldReloadHomeAfterConfigChange = ref(false)
  const showOperationDiagnostics = import.meta.env.DEV
  const operationDiagnostics = ref<string[]>([])
  const OPERATION_DIAGNOSTICS_LIMIT = 120

  let fetchDefaultSqlitePathHook = () => {}
  let ensureCurrentSqlitePathNotCloudHook = (_reason: string) => {}

  function registerPathHooks(hooks: {
    fetchDefaultSqlitePath: () => void
    ensureCurrentSqlitePathNotCloud: (reason: string) => void
  }) {
    fetchDefaultSqlitePathHook = hooks.fetchDefaultSqlitePath
    ensureCurrentSqlitePathNotCloudHook = hooks.ensureCurrentSqlitePathNotCloud
  }

  function handleBack() {
    emit('back', { reloadHome: shouldReloadHomeAfterConfigChange.value })
  }

  function appendOperationDiagnostic(message: string) {
    if (!showOperationDiagnostics) return
    const timestamp = new Date().toLocaleTimeString('zh-CN', { hour12: false })
    operationDiagnostics.value = [
      ...operationDiagnostics.value.slice(-(OPERATION_DIAGNOSTICS_LIMIT - 1)),
      `[${timestamp}] ${message}`,
    ]
  }

  function resetOperationDiagnostics(operation: '导入配置' | '保存配置') {
    if (!showOperationDiagnostics) return
    operationDiagnostics.value = []
    appendOperationDiagnostic(`开始${operation}`)
  }

  function clearOperationDiagnostics() {
    if (!showOperationDiagnostics) return
    operationDiagnostics.value = []
  }

  async function copyOperationDiagnostics() {
    if (!showOperationDiagnostics) return
    if (!operationDiagnostics.value.length) return
    await copyToClipboard(operationDiagnostics.value.join('\n'))
  }

  function updateFormFromConfig(config: AppConfig) {
    // 先重置响应式状态，确保导入配置能够完整覆盖旧值。
    localBookPath.value = ''
    Object.assign(gatewayConfig, {
      base_url: '',
      access_token: '',
    })
    sqlitePath.value = ''

    // 更新系统配置。
    if (config.system) {
      language.value = config.system.language
      themeMode.value = config.system.theme as 'system' | 'light' | 'dark'
      logLevel.value = config.system.log_level
      enableDebugTools.value = config.system.enable_debug_tools ?? false
      autoStartStudyTimer.value = config.system.auto_start_study_timer ?? false
      enableAutoCheck.value = config.system.enable_auto_check
      checkIntervalMins.value = config.system.check_interval_mins
    }

    // 更新图书来源配置。
    if (config.book_source) {
      sourceType.value = config.book_source.type
      if (config.book_source.type === 'Local') {
        localBookPath.value = config.book_source.details.path
      }
    } else {
      sourceType.value = 'Local'
    }

    // 更新数据库配置。
    if (config.database) {
      dbType.value = config.database.type
      if (config.database.type === 'SQLite') {
        sqlitePath.value = String(config.database.details.path || '').trim()
        if (!sqlitePath.value) {
          fetchDefaultSqlitePathHook()
        } else {
          ensureCurrentSqlitePathNotCloudHook('更新配置表单')
        }
      }
    } else {
      dbType.value = 'SQLite'
      // 已有 SQLite 路径时不使用默认值覆盖。
      if (!sqlitePath.value) {
        fetchDefaultSqlitePathHook()
      }
    }
    if (config.cloudflare_gateway) {
      gatewayConfig.base_url = config.cloudflare_gateway.base_url
      gatewayConfig.access_token = config.cloudflare_gateway.access_token
    }
    gatewayConfigurationRequired.value = config.gateway_configuration_required ?? false
  }

  function sanitizeErrorMessage(err: unknown): string {
    return String(err).replace(/https?:\/\/[^\s]+/g, '[URL]')
  }

  /** 为异步操作增加统一超时错误。
   * @param promiseOrValue - 需要等待的异步操作。
   * @param timeoutMs - 超时时间。
   * @param label - 错误信息中的操作名称。
   */
  function withTimeout<T>(
    promiseOrValue: Promise<T> | T,
    timeoutMs: number,
    label: string,
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new Error(`${label} 超时（${Math.ceil(timeoutMs / 1000)} 秒）`))
      }, timeoutMs)

      Promise.resolve(promiseOrValue)
        .then((result) => {
          clearTimeout(timer)
          resolve(result)
        })
        .catch((err) => {
          clearTimeout(timer)
          reject(err)
        })
    })
  }

  function buildConfigFromForm(): AppConfig {
    return {
      system: {
        language: language.value,
        theme: themeMode.value as 'system' | 'light' | 'dark',
        log_level: logLevel.value as any,
        enable_debug_tools: enableDebugTools.value,
        auto_start_study_timer: autoStartStudyTimer.value,
        enable_auto_check: enableAutoCheck.value,
        check_interval_mins: checkIntervalMins.value,
      },
      book_source: getCurrentBookSource(),
      database: getCurrentDatabase(),
      cloudflare_gateway: isCloudConfigured.value
        ? {
            base_url: gatewayConfig.base_url.trim().replace(/\/+$/, '') + '/',
            access_token: gatewayConfig.access_token,
          }
        : null,
      gateway_configuration_required: false,
    }
  }

  function getCurrentBookSource(): BookSource | null {
    if (sourceType.value === 'Local') {
      return {
        type: 'Local',
        details: { path: localBookPath.value },
      }
    } else {
      return {
        type: 'CloudflareGateway',
        details: {},
      }
    }
  }

  function getCurrentDatabase(): DatabaseConnection | null {
    if (dbType.value === 'SQLite') {
      return {
        type: 'SQLite',
        details: {
          path: sqlitePath.value.trim(),
        },
      }
    } else {
      return {
        type: 'CloudflareGateway',
        details: {},
      }
    }
  }

  return {
    initialConfig,
    t,
    locale,
    setTheme,
    appStore,
    token,
    activeTab,
    currentTab,
    debugFeaturesAvailable,
    showBackButton,
    language,
    themeMode,
    logLevel,
    enableDebugTools,
    autoStartStudyTimer,
    isCloudConfigured,
    enableAutoCheck,
    checkIntervalMins,
    sourceType,
    localBookPath,
    gatewayConfig,
    gatewayConfigurationRequired,
    dbType,
    sqlitePath,
    isTesting,
    isExporting,
    isImporting,
    isSaving,
    shouldReloadHomeAfterConfigChange,
    showOperationDiagnostics,
    operationDiagnostics,
    handleBack,
    appendOperationDiagnostic,
    resetOperationDiagnostics,
    clearOperationDiagnostics,
    copyOperationDiagnostics,
    copyToClipboard,
    updateFormFromConfig,
    sanitizeErrorMessage,
    withTimeout,
    buildConfigFromForm,
    getCurrentBookSource,
    getCurrentDatabase,
    registerPathHooks,
    emit,
  }
}

export type ConfigPageContext = ReturnType<typeof createConfigPageContext>
