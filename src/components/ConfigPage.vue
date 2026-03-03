<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { info, error, debug, warn } from '@tauri-apps/plugin-log'
import type {
  AppConfig,
  BookSource,
  BookSourceType,
  DatabaseConnection,
  DatabaseType,
} from '../types'
import { useI18n } from 'vue-i18n'
import { useTheme } from '../composables/useTheme'
import { useAppStore } from '../stores/app'
import {
  SettingOutlined,
  BookOutlined,
  DatabaseOutlined,
  HomeOutlined,
  DownloadOutlined,
  UploadOutlined,
} from '@ant-design/icons-vue'
import { message as antMessage, theme, Modal } from 'ant-design-vue'
import {
  saveConfig,
  exportConfig,
  importConfig,
  validateLocalBookSource,
  initializeDatabase,
  getDefaultSqlitePath,
  resolveSqlitePath as resolveSqlitePathFromApi,
  testDatabaseConnection,
  testR2Connection,
} from '../lib/api'
import SystemSettings from './config/SystemSettings.vue'
import BookSourceSettings from './config/BookSourceSettings.vue'
import DatabaseSettings from './config/DatabaseSettings.vue'

const { t, locale } = useI18n()
const { setTheme } = useTheme()
const appStore = useAppStore()
const { useToken } = theme
const { token } = useToken()

antMessage.config({
  top: '40px',
  duration: 3,
  maxCount: 3,
})

const props = defineProps<{
  initialConfig?: AppConfig
  allowBack?: boolean
}>()

const emit = defineEmits<{
  (e: 'config-saved', config: AppConfig): void
  (e: 'config-imported'): void
  (e: 'back', options?: { reloadHome?: boolean }): void
}>()

const activeTab = ref<string[]>(['system'])

const currentTab = computed(() => activeTab.value[0])
const debugFeaturesAvailable = __DEBUG_FEATURES__

// System Config
const language = ref(props.initialConfig?.system?.language || 'en')
const themeMode = ref(props.initialConfig?.system?.theme || 'system')
const logLevel = ref(props.initialConfig?.system?.log_level || 'info')
const enableDebugTools = ref(props.initialConfig?.system?.enable_debug_tools ?? false)
const isCloudConfigured = computed(
  () => sourceType.value === 'CloudflareR2' || dbType.value === 'CloudflareD1',
)
const enableAutoCheck = ref(props.initialConfig?.system?.enable_auto_check ?? true)
const checkIntervalMins = ref(props.initialConfig?.system?.check_interval_mins || 5)

// Book Source Config
const sourceType = ref<BookSourceType>(props.initialConfig?.book_source?.type || 'Local')
const localBookPath = ref(
  props.initialConfig?.book_source?.type === 'Local'
    ? props.initialConfig.book_source.details.path
    : '',
)
const r2Config = reactive({
  account_id:
    props.initialConfig?.book_source?.type === 'CloudflareR2'
      ? props.initialConfig.book_source.details.account_id
      : '',
  bucket_name:
    props.initialConfig?.book_source?.type === 'CloudflareR2'
      ? props.initialConfig.book_source.details.bucket_name
      : '',
  access_key_id:
    props.initialConfig?.book_source?.type === 'CloudflareR2'
      ? props.initialConfig.book_source.details.access_key_id
      : '',
  secret_access_key:
    props.initialConfig?.book_source?.type === 'CloudflareR2'
      ? props.initialConfig.book_source.details.secret_access_key
      : '',
  public_url:
    props.initialConfig?.book_source?.type === 'CloudflareR2'
      ? props.initialConfig.book_source.details.public_url || ''
      : '',
})

// Database Config
const dbType = ref<DatabaseType>(props.initialConfig?.database?.type || 'SQLite')
const sqlitePath = ref('')
const d1Config = reactive({
  account_id:
    props.initialConfig?.database?.type === 'CloudflareD1'
      ? props.initialConfig.database.details.account_id
      : '',
  database_id:
    props.initialConfig?.database?.type === 'CloudflareD1'
      ? props.initialConfig.database.details.database_id
      : '',
  api_token:
    props.initialConfig?.database?.type === 'CloudflareD1'
      ? props.initialConfig.database.details.api_token
      : '',
})

const windowsAbsolutePathRegex = /^[a-zA-Z]:[\\/]/
const uncAbsolutePathRegex = /^\\\\[^\\]+\\[^\\]+/
const unixAbsolutePathRegex = /^\//
const fileUrlRegex = /^file:\/\//i

function hasFileExtension(path: string): boolean {
  const lastSegment = path.split(/[\\/]/).pop() || ''
  return /\.[^./\\]+$/.test(lastSegment)
}

function normalizeSqlitePath(path: string): string {
  const trimmed = path.trim()
  if (!trimmed) {
    return ''
  }
  if (hasFileExtension(trimmed)) {
    return trimmed
  }
  return `${trimmed}.db`
}

function isAbsolutePath(path: string): boolean {
  if (fileUrlRegex.test(path)) {
    return true
  }
  return (
    windowsAbsolutePathRegex.test(path) ||
    uncAbsolutePathRegex.test(path) ||
    unixAbsolutePathRegex.test(path)
  )
}

function isAppleMobileDevice(): boolean {
  const ua = navigator.userAgent || ''
  const isLegacyIos = /iPad|iPhone|iPod/i.test(ua)
  const isModernIpad = navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1
  return isLegacyIos || isModernIpad
}

const cloudPathMarkers = [
  'mobile documents',
  'com~apple~clouddocs',
  'onedrive -',
  'onedrive',
  'dropbox',
  'google drive',
  'googledrive',
]

function isCloudSyncedPath(path: string): boolean {
  const normalized = normalizeDialogSelectedPath(path).toLowerCase()
  return cloudPathMarkers.some((marker) => normalized.includes(marker))
}

function normalizeDialogSelectedPath(path: string): string {
  const trimmed = path.trim()
  if (!fileUrlRegex.test(trimmed)) {
    return trimmed
  }

  try {
    const parsed = new URL(trimmed)
    const decoded = decodeURIComponent(parsed.pathname)
    // Windows file URL may be /C:/foo/bar
    if (/^\/[a-zA-Z]:\//.test(decoded)) {
      return decoded.slice(1)
    }
    return decoded
  } catch {
    return trimmed
  }
}

async function resolveDefaultSqlitePathForFallback(reason: string): Promise<string | null> {
  appendOperationDiagnostic(`SQLite 云盘路径命中，尝试回退默认路径（${reason}）`)
  try {
    const defaultPath = await withTimeout(getDefaultSqlitePath(), 8_000, '获取默认 SQLite 路径')
    const normalizedDefaultPath = normalizeSqlitePath(defaultPath || '')
    if (!normalizedDefaultPath) {
      appendOperationDiagnostic('默认 SQLite 路径为空，无法回退')
      antMessage.error(
        t('config.sqliteCloudPathFallbackFailed' as any) || 'SQLite 云盘路径回退默认路径失败',
      )
      return null
    }
    if (isCloudSyncedPath(normalizedDefaultPath)) {
      appendOperationDiagnostic(`默认 SQLite 路径仍命中云盘目录: ${normalizedDefaultPath}`)
      antMessage.error(
        t('config.sqliteCloudPathFallbackFailed' as any) || 'SQLite 云盘路径回退默认路径失败',
      )
      return null
    }
    sqlitePath.value = normalizedDefaultPath
    antMessage.warning(
      t('config.sqliteCloudPathAutoFallback' as any) ||
        '检测到云盘 SQLite 路径，已自动回退到默认本地路径',
    )
    appendOperationDiagnostic(`SQLite 路径已回退默认路径: ${normalizedDefaultPath}`)
    return normalizedDefaultPath
  } catch (err) {
    appendOperationDiagnostic(`回退默认 SQLite 路径失败: ${sanitizeErrorMessage(err)}`)
    antMessage.error(
      t('config.sqliteCloudPathFallbackFailed' as any) || 'SQLite 云盘路径回退默认路径失败',
    )
    return null
  }
}

async function enforceSqliteCloudPathPolicy(
  rawPath: string,
  reason: string,
): Promise<string | null> {
  const normalizedPath = normalizeDialogSelectedPath(rawPath).trim()
  if (!normalizedPath) {
    return ''
  }
  if (!isCloudSyncedPath(normalizedPath)) {
    return normalizedPath
  }
  return resolveDefaultSqlitePathForFallback(reason)
}

async function ensureConfigSqlitePathNotCloud(config: AppConfig, reason: string): Promise<boolean> {
  if (config.database?.type !== 'SQLite') {
    return true
  }
  const rawPath = String(config.database.details.path || '').trim()
  const safePath = await enforceSqliteCloudPathPolicy(rawPath, reason)
  if (safePath === null) {
    return false
  }
  config.database.details.path = safePath
  sqlitePath.value = safePath
  return true
}

async function ensureCurrentSqlitePathNotCloud(reason: string): Promise<boolean> {
  const safePath = await enforceSqliteCloudPathPolicy(sqlitePath.value, reason)
  if (safePath === null) {
    return false
  }
  sqlitePath.value = safePath
  return true
}

function getParentPath(path: string): string {
  const normalized = path.replace(/[\\/]+$/, '')
  const index = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'))
  if (index < 0) return normalized
  if (index === 0) return normalized.slice(0, 1)
  return normalized.slice(0, index)
}

function buildSelectionMarkerFileName(prefix: string): string {
  const token = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
  return `${prefix}-${token}.tmp`
}

async function pickFolderPathOnAppleMobile(
  currentPath: string,
  markerPrefix: string,
): Promise<string> {
  try {
    const selectedDirectory = await open({
      directory: true,
      recursive: true,
      multiple: false,
      defaultPath: currentPath || undefined,
      pickerMode: 'document',
      fileAccessMode: 'scoped',
    })
    if (selectedDirectory && typeof selectedDirectory === 'string') {
      return normalizeDialogSelectedPath(selectedDirectory)
    }
  } catch (err) {
    warn(`iPadOS 目录选择失败，尝试目录定位兜底: ${sanitizeErrorMessage(err)}`)
  }

  try {
    const markerFileName = buildSelectionMarkerFileName(markerPrefix)
    const locatedPath = await save({
      defaultPath: markerFileName,
    })
    if (locatedPath && typeof locatedPath === 'string') {
      const parent = getParentPath(normalizeDialogSelectedPath(locatedPath))
      if (parent) {
        return parent
      }
    }
  } catch (err) {
    warn(`iPadOS 目录定位失败，尝试文件兜底: ${sanitizeErrorMessage(err)}`)
  }

  try {
    const selectedFile = await open({
      multiple: false,
      defaultPath: currentPath || undefined,
      pickerMode: 'document',
      fileAccessMode: 'scoped',
    })
    if (selectedFile && typeof selectedFile === 'string') {
      const parent = getParentPath(normalizeDialogSelectedPath(selectedFile))
      if (parent) {
        return parent
      }
    }
  } catch (err) {
    warn(`iPadOS 文件兜底失败: ${sanitizeErrorMessage(err)}`)
  }

  return ''
}

async function pickSqliteFilePathOnAppleMobile(currentPath: string): Promise<string> {
  try {
    const selectedFile = await open({
      multiple: false,
      filters: [
        {
          name: 'SQLite Database',
          extensions: ['db', 'sqlite', 'sqlite3'],
        },
      ],
      defaultPath: currentPath || undefined,
      pickerMode: 'document',
      fileAccessMode: 'scoped',
    })
    if (selectedFile && typeof selectedFile === 'string') {
      return normalizeDialogSelectedPath(selectedFile)
    }
  } catch (err) {
    warn(`iPadOS SQLite 文件选择失败，将尝试新建路径: ${sanitizeErrorMessage(err)}`)
  }

  try {
    const selected = await save({
      filters: [
        {
          name: 'SQLite Database',
          extensions: ['db', 'sqlite', 'sqlite3'],
        },
      ],
      defaultPath: currentPath || 'english-in-use.db',
    })
    if (selected && typeof selected === 'string') {
      return normalizeDialogSelectedPath(selected)
    }
  } catch (err) {
    warn(`iPadOS SQLite 路径新建失败: ${sanitizeErrorMessage(err)}`)
  }

  return ''
}

async function resolveSqlitePathForUsage(rawPath: string): Promise<string> {
  const trimmed = rawPath.trim()
  if (!trimmed) return ''

  try {
    const resolved = await withTimeout(
      resolveSqlitePathFromApi(trimmed),
      10_000,
      '解析 SQLite 路径',
    )
    return normalizeSqlitePath(resolved || trimmed)
  } catch (err) {
    const errMsg = sanitizeErrorMessage(err)
    appendOperationDiagnostic(`解析 SQLite 路径失败，回退本地规则: ${errMsg}`)
    warn(`解析 SQLite 路径失败，回退本地规则: ${errMsg}`)
    return normalizeSqlitePath(trimmed)
  }
}

async function fetchDefaultSqlitePath() {
  try {
    const path = await getDefaultSqlitePath()
    sqlitePath.value = normalizeSqlitePath(path)
  } catch (err) {
    console.error('Failed to get default sqlite path:', err)
  }
}

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    antMessage.success(t('common.copied' as any) || 'Copied to clipboard')
  } catch (err) {
    antMessage.error('Failed to copy')
  }
}

watch(dbType, (newType) => {
  if (newType === 'SQLite') {
    if (!sqlitePath.value) {
      fetchDefaultSqlitePath()
      return
    }
    void ensureCurrentSqlitePathNotCloud('切换到 SQLite')
  }
})

// Initial fetch if type is SQLite
if (dbType.value === 'SQLite') {
  if (props.initialConfig?.database?.type === 'SQLite') {
    sqlitePath.value = String(props.initialConfig.database.details.path || '').trim()
  } else {
    fetchDefaultSqlitePath()
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

if (dbType.value === 'SQLite' && sqlitePath.value.trim()) {
  void ensureCurrentSqlitePathNotCloud('加载现有配置')
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
  // Reset reactive state to ensure full overwrite
  localBookPath.value = ''
  Object.assign(r2Config, {
    account_id: '',
    bucket_name: '',
    access_key_id: '',
    secret_access_key: '',
    public_url: '',
  })
  Object.assign(d1Config, {
    account_id: '',
    database_id: '',
    api_token: '',
  })
  sqlitePath.value = ''

  // Update system config
  if (config.system) {
    language.value = config.system.language
    themeMode.value = config.system.theme as 'system' | 'light' | 'dark'
    logLevel.value = config.system.log_level
    enableDebugTools.value = config.system.enable_debug_tools ?? false
    enableAutoCheck.value = config.system.enable_auto_check
    checkIntervalMins.value = config.system.check_interval_mins
  }

  // Update book source config
  if (config.book_source) {
    sourceType.value = config.book_source.type
    if (config.book_source.type === 'Local') {
      localBookPath.value = config.book_source.details.path
    } else if (config.book_source.type === 'CloudflareR2') {
      const details = config.book_source.details
      r2Config.account_id = details.account_id
      r2Config.bucket_name = details.bucket_name
      r2Config.access_key_id = details.access_key_id
      r2Config.secret_access_key = details.secret_access_key
      r2Config.public_url = details.public_url || ''
    }
  } else {
    sourceType.value = 'Local'
  }

  // Update database config
  if (config.database) {
    dbType.value = config.database.type
    if (config.database.type === 'SQLite') {
      sqlitePath.value = String(config.database.details.path || '').trim()
      if (!sqlitePath.value) {
        fetchDefaultSqlitePath()
      } else {
        void ensureCurrentSqlitePathNotCloud('更新配置表单')
      }
    } else if (config.database.type === 'CloudflareD1') {
      const details = config.database.details
      d1Config.account_id = details.account_id
      d1Config.database_id = details.database_id
      d1Config.api_token = details.api_token
    }
  } else {
    dbType.value = 'SQLite'
    // Do not reset sqlitePath if we already have it
    if (!sqlitePath.value) {
      fetchDefaultSqlitePath()
    }
  }
}

function sanitizeErrorMessage(err: unknown): string {
  return String(err).replace(/https?:\/\/[^\s]+/g, '[URL]')
}

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

async function checkCloudConnections(config: AppConfig): Promise<{ isD1Connected: boolean }> {
  appendOperationDiagnostic('开始检查云连接')
  appStore.setGlobalLoadingMessage(t('config.checkingConnections' as any) || '正在检查云连接...')
  appStore.setGlobalLoadingProgress(45)
  let isD1Connected = config.database?.type !== 'CloudflareD1'
  const connectionTimeoutMs = 20_000

  if (config.book_source?.type === 'CloudflareR2') {
    appStore.setGlobalLoadingMessage('正在检查 R2 连接...')
    appStore.setGlobalLoadingProgress(55)
    appendOperationDiagnostic('开始检查 R2 连接')
    info('开始检查 R2 连接')
    try {
      await withTimeout(testR2Connection(config.book_source), connectionTimeoutMs, 'R2 连接检查')
      appendOperationDiagnostic('R2 连接检查完成')
      info('R2 连接检查完成')
    } catch (err) {
      const errMsg = sanitizeErrorMessage(err)
      appendOperationDiagnostic(`R2 连接检查失败（忽略并继续）: ${errMsg}`)
      warn(`R2 连接检查失败（不影响配置保存/导入）: ${errMsg}`)
      antMessage.warning(t('config.connectionCheckWarning', { service: 'R2', error: errMsg }))
    }
  }

  if (config.database?.type === 'CloudflareD1') {
    appStore.setGlobalLoadingMessage('正在检查 D1 连接...')
    appStore.setGlobalLoadingProgress(65)
    appendOperationDiagnostic('开始检查 D1 连接')
    info('开始检查 D1 连接')
    try {
      await withTimeout(testDatabaseConnection(config.database), connectionTimeoutMs, 'D1 连接检查')
      isD1Connected = true
      appendOperationDiagnostic('D1 连接检查完成')
      info('D1 连接检查完成')
    } catch (err) {
      const errMsg = sanitizeErrorMessage(err)
      isD1Connected = false
      appendOperationDiagnostic(`D1 连接检查失败（忽略并继续）: ${errMsg}`)
      warn(`D1 连接检查失败（不影响配置保存/导入）: ${errMsg}`)
      antMessage.warning(t('config.connectionCheckWarning', { service: 'D1', error: errMsg }))
    }
  }

  appendOperationDiagnostic(`云连接检查结束，D1可用: ${isD1Connected ? '是' : '否'}`)
  return { isD1Connected }
}

async function initializeDatabaseIfNeeded(config: AppConfig, isD1Connected: boolean) {
  const databaseType = config.database?.type
  if (!databaseType) {
    appendOperationDiagnostic('跳过数据库初始化（未配置数据库）')
    return
  }

  if (databaseType === 'CloudflareD1' && !isD1Connected) {
    appendOperationDiagnostic('跳过数据库初始化（D1 不可用）')
    return
  }

  try {
    appStore.setGlobalLoadingMessage(
      t('config.initializingDatabase' as any) || '正在初始化数据库...',
    )
    appStore.setGlobalLoadingProgress(80)
    appendOperationDiagnostic(`开始初始化数据库: ${databaseType}`)
    await withTimeout(initializeDatabase(), 20_000, '数据库初始化')
    appendOperationDiagnostic(`数据库初始化完成: ${databaseType}`)
    info(`${databaseType} 数据库初始化完成`)
  } catch (err) {
    const errMsg = sanitizeErrorMessage(err)
    if (
      databaseType === 'SQLite' &&
      isAppleMobileDevice() &&
      /operation not permitted|permission denied|os error 1/i.test(String(err))
    ) {
      appendOperationDiagnostic('检测到 iPadOS SQLite 权限错误，尝试切换默认路径并重试初始化')
      try {
        const defaultPath = await withTimeout(getDefaultSqlitePath(), 8_000, '获取默认 SQLite 路径')
        const normalizedDefaultPath = normalizeSqlitePath(defaultPath || '')
        if (normalizedDefaultPath && config.database?.type === 'SQLite') {
          config.database.details.path = normalizedDefaultPath
          sqlitePath.value = normalizedDefaultPath
          await withTimeout(saveConfig(config), 15_000, '回写默认 SQLite 路径')
          await withTimeout(initializeDatabase(), 20_000, '数据库初始化重试')
          appendOperationDiagnostic('SQLite 默认路径重试初始化成功')
          antMessage.warning('iPadOS 本地目录权限受限，已自动切换到应用默认数据库路径')
          return
        }
      } catch (retryErr) {
        appendOperationDiagnostic(`SQLite 默认路径重试失败: ${sanitizeErrorMessage(retryErr)}`)
      }
    }

    appendOperationDiagnostic(`数据库初始化失败（忽略并继续）: ${errMsg}`)
    warn(`数据库初始化失败（不影响配置保存/导入）: ${errMsg}`)
    antMessage.warning(t('config.databaseInitWarning', { error: errMsg }))
  }
}

async function ensureSqlitePathForImportIfNeeded(config: AppConfig): Promise<boolean> {
  if (config.database?.type !== 'SQLite') {
    return true
  }

  const currentPath = String(config.database.details.path || '').trim()
  if (currentPath) {
    const cloudPathAllowed = await ensureConfigSqlitePathNotCloud(config, '导入配置')
    if (!cloudPathAllowed) {
      appendOperationDiagnostic('导入配置失败：SQLite 路径位于云盘目录且回退失败')
      return false
    }
    return true
  }

  appendOperationDiagnostic('导入配置缺少 SQLite 路径，尝试回填默认路径')
  try {
    const defaultPath = await withTimeout(getDefaultSqlitePath(), 8_000, '获取默认 SQLite 路径')
    const normalizedDefaultPath = normalizeSqlitePath(defaultPath || '')
    if (!normalizedDefaultPath) {
      appendOperationDiagnostic('获取默认 SQLite 路径失败：返回为空')
      antMessage.error(t('config.sqlitePathRequired' as any) || 'SQLite 数据库路径不能为空')
      return false
    }
    if (isCloudSyncedPath(normalizedDefaultPath)) {
      appendOperationDiagnostic(`默认 SQLite 路径命中云盘目录: ${normalizedDefaultPath}`)
      antMessage.error(
        t('config.sqliteCloudPathFallbackFailed' as any) || 'SQLite 云盘路径回退默认路径失败',
      )
      return false
    }
    config.database.details.path = normalizedDefaultPath
    sqlitePath.value = normalizedDefaultPath
    appendOperationDiagnostic(`已回填默认 SQLite 路径: ${normalizedDefaultPath}`)
    antMessage.info(
      t('config.sqlitePathAutoFilled' as any) || 'SQLite 路径为空，已自动回填默认路径',
    )
    return true
  } catch (err) {
    const errMsg = sanitizeErrorMessage(err)
    appendOperationDiagnostic(`回填默认 SQLite 路径失败: ${errMsg}`)
    antMessage.error(t('config.sqlitePathRequired' as any) || 'SQLite 数据库路径不能为空')
    return false
  }
}

async function validateSqlitePathIfNeeded(config: AppConfig): Promise<boolean> {
  if (config.database?.type !== 'SQLite') {
    return true
  }

  let workingPath = String(config.database.details.path || '').trim()
  if (!workingPath) {
    appendOperationDiagnostic('SQLite 路径校验失败：路径为空')
    antMessage.error(t('config.sqlitePathRequired' as any) || 'SQLite 数据库路径不能为空')
    return false
  }

  const safePath = await enforceSqliteCloudPathPolicy(workingPath, '保存前校验')
  if (safePath === null) {
    appendOperationDiagnostic('SQLite 路径校验失败：命中云盘目录且回退失败')
    return false
  }
  workingPath = safePath.trim()

  if (!workingPath) {
    appendOperationDiagnostic('SQLite 路径校验失败：路径为空')
    antMessage.error(t('config.sqlitePathRequired' as any) || 'SQLite 数据库路径不能为空')
    return false
  }

  if (!isAbsolutePath(workingPath)) {
    appendOperationDiagnostic(`SQLite 路径校验失败：非绝对路径 (${workingPath})`)
    antMessage.error(
      t('config.sqlitePathAbsoluteRequired' as any) || 'SQLite 数据库路径必须是绝对路径',
    )
    return false
  }

  const resolvedPath = await resolveSqlitePathForUsage(workingPath)
  if (!resolvedPath) {
    appendOperationDiagnostic('SQLite 路径解析失败：结果为空')
    antMessage.error(t('config.sqlitePathRequired' as any) || 'SQLite 数据库路径不能为空')
    return false
  }

  config.database.details.path = resolvedPath
  sqlitePath.value = resolvedPath
  return true
}

async function validateLocalBookSourceIfNeeded(config: AppConfig): Promise<boolean> {
  if (config.book_source?.type !== 'Local') {
    return true
  }

  const localPath = config.book_source.details.path?.trim()
  if (!localPath) {
    appendOperationDiagnostic('跳过本地图书目录检查（本地路径为空）')
    return true
  }

  appStore.setGlobalLoadingMessage('正在校验本地图书目录...')
  appStore.setGlobalLoadingProgress(30)
  appendOperationDiagnostic(`开始校验本地图书目录: ${localPath}`)

  const validation = await withTimeout(
    validateLocalBookSource(localPath),
    12_000,
    '本地图书目录校验',
  )

  validation.warnings.forEach((warningText) => {
    appendOperationDiagnostic(`本地目录告警: ${warningText}`)
    antMessage.warning(warningText)
  })

  if (!validation.ok) {
    const detail = validation.errors.join('；') || '本地图书目录校验失败'
    appendOperationDiagnostic(`本地目录校验失败: ${detail}`)
    antMessage.error(detail)
    return false
  }

  appendOperationDiagnostic('本地图书目录校验通过')
  return true
}

async function ensureAppleMobileLocalAuthorizationIfNeeded(config: AppConfig): Promise<boolean> {
  if (!isAppleMobileDevice()) {
    return true
  }

  if (config.book_source?.type === 'Local') {
    const currentLocalPath = String(config.book_source.details.path || '').trim()
    if (currentLocalPath) {
      appendOperationDiagnostic('iPadOS 本地图书目录授权开始')
      const authorizedFolder = await pickFolderPathOnAppleMobile(
        currentLocalPath,
        'book-folder-auth',
      )
      if (!authorizedFolder) {
        appendOperationDiagnostic('iPadOS 本地图书目录授权取消')
        antMessage.warning('请先完成本地图书目录授权')
        return false
      }
      config.book_source.details.path = authorizedFolder
      localBookPath.value = authorizedFolder
      appendOperationDiagnostic(`iPadOS 本地图书目录授权完成: ${authorizedFolder}`)
    }
  }

  if (config.database?.type === 'SQLite') {
    const currentSqlitePath = String(config.database.details.path || '').trim()
    if (currentSqlitePath) {
      appendOperationDiagnostic('iPadOS SQLite 文件授权开始')
      const authorizedSqlitePath = await pickSqliteFilePathOnAppleMobile(currentSqlitePath)
      if (!authorizedSqlitePath) {
        appendOperationDiagnostic('iPadOS SQLite 文件授权取消')
        antMessage.warning('请先完成 SQLite 文件授权')
        return false
      }
      config.database.details.path = authorizedSqlitePath
      sqlitePath.value = authorizedSqlitePath
      const cloudPathAllowed = await ensureConfigSqlitePathNotCloud(
        config,
        'iPadOS SQLite 文件授权',
      )
      if (!cloudPathAllowed) {
        appendOperationDiagnostic('iPadOS SQLite 文件授权失败：命中云盘目录且回退失败')
        return false
      }
      appendOperationDiagnostic(`iPadOS SQLite 文件授权完成: ${authorizedSqlitePath}`)
    }
  }

  return true
}

async function handleSave() {
  isSaving.value = true
  resetOperationDiagnostics('保存配置')
  info('用户触发了保存配置操作')
  try {
    appStore.startGlobalLoading(t('config.savingConfig' as any) || '正在保存配置...')
    appStore.setGlobalLoadingProgress(10)

    const config: AppConfig = {
      system: {
        language: language.value,
        theme: themeMode.value as 'system' | 'light' | 'dark',
        log_level: logLevel.value as any,
        enable_debug_tools: enableDebugTools.value,
        enable_auto_check: enableAutoCheck.value,
        check_interval_mins: checkIntervalMins.value,
      },
      book_source: getCurrentBookSource(),
      database: getCurrentDatabase(),
    }

    const appleAuthorized = await ensureAppleMobileLocalAuthorizationIfNeeded(config)
    if (!appleAuthorized) return

    const localSourceValid = await validateLocalBookSourceIfNeeded(config)
    if (!localSourceValid) return

    const sqlitePathValid = await validateSqlitePathIfNeeded(config)
    if (!sqlitePathValid) return

    appendOperationDiagnostic('开始写入配置文件')
    await withTimeout(saveConfig(config), 15_000, '保存配置')
    appStore.setGlobalLoadingProgress(35)
    appendOperationDiagnostic('配置文件写入完成')
    info('配置保存成功')

    const { isD1Connected } = await withTimeout(
      checkCloudConnections(config),
      45_000,
      '云连接检查流程',
    )
    await withTimeout(initializeDatabaseIfNeeded(config, isD1Connected), 30_000, '数据库初始化流程')
    appStore.setGlobalLoadingProgress(95)

    locale.value = language.value
    setTheme(themeMode.value as any)

    shouldReloadHomeAfterConfigChange.value = true
    emit('config-saved', config)

    Modal.confirm({
      title: t('config.savedSuccess'),
      content: t('config.returnToBookListConfirm' as any) || '保存成功，是否返回书籍列表？',
      okText: t('common.yes' as any) || '是',
      cancelText: t('common.no' as any) || '否',
      onOk: () => {
        info('用户选择返回书籍列表')
        handleBack()
      },
    })
  } catch (err) {
    appendOperationDiagnostic(`保存流程失败: ${sanitizeErrorMessage(err)}`)
    error(`保存配置失败: ${err}`)
    antMessage.error(t('config.saveError', { error: err }))
  } finally {
    appStore.setGlobalLoadingProgress(null)
    appStore.stopGlobalLoading()
    isSaving.value = false
  }
}

async function handleExport() {
  isExporting.value = true
  info('用户触发了导出配置操作')
  try {
    const config: AppConfig = {
      system: {
        language: language.value,
        theme: themeMode.value as 'system' | 'light' | 'dark',
        log_level: logLevel.value as any,
        enable_debug_tools: enableDebugTools.value,
        enable_auto_check: enableAutoCheck.value,
        check_interval_mins: checkIntervalMins.value,
      },
      book_source: getCurrentBookSource(),
      database: getCurrentDatabase(),
    }

    let filePath = await save({
      filters: [
        {
          name: 'TOML Configuration',
          extensions: ['toml'],
        },
      ],
      defaultPath: 'english-in-use-config.toml',
    })

    if (filePath) {
      debug(`导出目标路径: ${filePath}`)
      // Ensure the file has .toml extension
      if (!filePath.toLowerCase().endsWith('.toml')) {
        filePath += '.toml'
      }
      await exportConfig(filePath, config)
      info('配置文件导出成功')
      antMessage.success(t('config.exportSuccess'))
    } else {
      debug('用户取消了导出操作')
    }
  } catch (err) {
    error(`导出配置失败: ${err}`)
    antMessage.error(t('config.exportError', { error: err }))
  } finally {
    isExporting.value = false
  }
}

async function handleImport() {
  isImporting.value = true
  resetOperationDiagnostics('导入配置')
  info('用户触发了导入配置操作')
  let globalLoadingStarted = false
  try {
    appendOperationDiagnostic('等待用户选择配置文件')
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'TOML Configuration',
          extensions: ['toml'],
        },
      ],
    })

    if (selected && typeof selected === 'string') {
      appStore.startGlobalLoading(t('config.importingConfig' as any) || '正在导入配置...')
      appStore.setGlobalLoadingProgress(10)
      globalLoadingStarted = true
      appendOperationDiagnostic(`已选择配置文件: ${selected}`)
      debug(`选择导入的文件: ${selected}`)
      appendOperationDiagnostic('开始读取导入配置')
      const config: AppConfig = await withTimeout(importConfig(selected), 15_000, '读取配置文件')
      config.system.enable_debug_tools = config.system.enable_debug_tools ?? false
      appStore.setGlobalLoadingProgress(25)
      appendOperationDiagnostic('导入配置读取完成')

      const appleAuthorized = await ensureAppleMobileLocalAuthorizationIfNeeded(config)
      if (!appleAuthorized) return

      const localSourceValid = await validateLocalBookSourceIfNeeded(config)
      if (!localSourceValid) return
      const sqlitePathPrepared = await ensureSqlitePathForImportIfNeeded(config)
      if (!sqlitePathPrepared) return
      const sqlitePathValid = await validateSqlitePathIfNeeded(config)
      if (!sqlitePathValid) return

      // Save to application config immediately (overwrite current config.toml)
      appendOperationDiagnostic('开始写入应用配置')
      await withTimeout(saveConfig(config), 15_000, '保存导入配置')
      appStore.setGlobalLoadingProgress(40)
      appendOperationDiagnostic('应用配置写入完成')
      info('配置文件导入并保存成功')

      const { isD1Connected } = await withTimeout(
        checkCloudConnections(config),
        45_000,
        '云连接检查流程',
      )
      await withTimeout(
        initializeDatabaseIfNeeded(config, isD1Connected),
        30_000,
        '数据库初始化流程',
      )
      appStore.setGlobalLoadingProgress(95)

      // Update form fields
      updateFormFromConfig(config)

      // Apply system settings immediately
      locale.value = language.value
      setTheme(themeMode.value as any)

      shouldReloadHomeAfterConfigChange.value = true
      emit('config-imported')
      emit('config-saved', config)

      Modal.confirm({
        title: t('config.importSuccess'),
        content: t('config.returnToBookListConfirm' as any) || '导入成功，是否返回书籍列表？',
        okText: t('common.yes' as any) || '是',
        cancelText: t('common.no' as any) || '否',
        onOk: () => {
          info('用户选择返回书籍列表 (导入后)')
          handleBack()
        },
      })
    } else {
      appendOperationDiagnostic('用户取消导入')
      debug('用户取消了导入操作')
    }
  } catch (err) {
    appendOperationDiagnostic(`导入流程失败: ${sanitizeErrorMessage(err)}`)
    error(`导入配置失败: ${err}`)
    antMessage.error(t('config.importError', { error: err }))
  } finally {
    appStore.setGlobalLoadingProgress(null)
    if (globalLoadingStarted) {
      appStore.stopGlobalLoading()
    }
    isImporting.value = false
  }
}

async function testConnection() {
  isTesting.value = true
  info(`正在测试连接, 当前标签页: ${currentTab.value}`)
  try {
    if (currentTab.value === 'books') {
      const source = getCurrentBookSource()
      if (!source || source.type !== 'CloudflareR2') {
        warn('尝试测试非 R2 类型的图书源连接')
        return
      }
      await testR2Connection(source)
      info('R2 连接测试成功 (前端反馈)')
      antMessage.success(t('config.testSuccess'))
    } else if (currentTab.value === 'database') {
      const connection = getCurrentDatabase()
      if (!connection) {
        warn('尝试测试未配置的数据库连接')
        return
      }
      if (connection.type === 'SQLite') {
        let rawPath = String(connection.details.path || '').trim()
        const safePath = await enforceSqliteCloudPathPolicy(rawPath, '连接测试')
        if (safePath === null) {
          appendOperationDiagnostic('SQLite 连接测试阻止：命中云盘目录且回退失败')
          return
        }
        rawPath = safePath.trim()
        if (!rawPath || !isAbsolutePath(rawPath)) {
          antMessage.error(
            t('config.sqlitePathAbsoluteRequired' as any) || 'SQLite 数据库路径必须是绝对路径',
          )
          return
        }
        connection.details.path = await resolveSqlitePathForUsage(rawPath)
        sqlitePath.value = connection.details.path
      }
      await testDatabaseConnection(connection)
      info('数据库连接测试成功 (前端反馈)')
      antMessage.success(t('config.testSuccess'))
    }
  } catch (err) {
    error(`连接测试失败: ${err}`)
    const errMsg = String(err).replace(/https?:\/\/[^\s]+/g, '[URL]')
    antMessage.error(t('config.connectionFailed', { error: errMsg }))
  } finally {
    isTesting.value = false
  }
}

async function selectBookFolder() {
  const isMobileApple = isAppleMobileDevice()
  if (isMobileApple) {
    const selectedFolder = await pickFolderPathOnAppleMobile(localBookPath.value, 'book-folder')
    if (selectedFolder) {
      localBookPath.value = selectedFolder
    } else {
      antMessage.warning('未完成目录选择，请重试')
    }
    return
  }

  try {
    const selected = await open({
      directory: true,
      recursive: true,
      multiple: false,
      defaultPath: localBookPath.value || undefined,
      pickerMode: 'document',
    })
    if (selected && typeof selected === 'string') {
      localBookPath.value = normalizeDialogSelectedPath(selected)
      return
    }
  } catch (err) {
    warn(`目录选择失败: ${sanitizeErrorMessage(err)}`)
  }
}

async function selectSqliteDatabasePath() {
  const isMobileApple = isAppleMobileDevice()
  if (isMobileApple) {
    const selectedPath = await pickSqliteFilePathOnAppleMobile(sqlitePath.value)
    if (selectedPath) {
      const safePath = await enforceSqliteCloudPathPolicy(selectedPath, '选择 SQLite 路径')
      if (safePath === null) return
      sqlitePath.value = await resolveSqlitePathForUsage(safePath)
      return
    }

    if (!sqlitePath.value.trim()) {
      try {
        const defaultPath = await getDefaultSqlitePath()
        sqlitePath.value = normalizeSqlitePath(defaultPath)
        antMessage.info('已回退到应用默认 SQLite 路径')
      } catch (err) {
        warn(`回退默认 SQLite 路径失败: ${sanitizeErrorMessage(err)}`)
      }
    } else {
      antMessage.warning('未完成 SQLite 文件授权，请重试')
    }
    return
  }

  try {
    const selected = await open({
      directory: true,
      recursive: true,
      multiple: false,
      defaultPath: sqlitePath.value || undefined,
      pickerMode: 'document',
    })

    if (selected && typeof selected === 'string') {
      const selectedPath = normalizeDialogSelectedPath(selected)
      const safePath = await enforceSqliteCloudPathPolicy(selectedPath, '选择 SQLite 路径')
      if (safePath === null) return
      sqlitePath.value = await resolveSqlitePathForUsage(safePath)
      return
    }
  } catch (err) {
    warn(`SQLite 目录选择失败，将尝试文件选择兜底: ${sanitizeErrorMessage(err)}`)
  }

  try {
    const selected = await save({
      filters: [
        {
          name: 'SQLite Database',
          extensions: ['db', 'sqlite', 'sqlite3'],
        },
      ],
      defaultPath: sqlitePath.value || undefined,
    })

    if (selected && typeof selected === 'string') {
      const selectedPath = normalizeDialogSelectedPath(selected)
      const safePath = await enforceSqliteCloudPathPolicy(selectedPath, '选择 SQLite 路径')
      if (safePath === null) return
      sqlitePath.value = await resolveSqlitePathForUsage(safePath)
    }
  } catch (err) {
    console.error('Failed to select sqlite path:', err)
  }
}

async function restoreDefaultSqlitePath() {
  await fetchDefaultSqlitePath()
}

function getCurrentBookSource(): BookSource | null {
  if (sourceType.value === 'Local') {
    return {
      type: 'Local',
      details: { path: localBookPath.value },
    }
  } else {
    return {
      type: 'CloudflareR2',
      details: {
        account_id: r2Config.account_id,
        bucket_name: r2Config.bucket_name,
        access_key_id: r2Config.access_key_id,
        secret_access_key: r2Config.secret_access_key,
        public_url: r2Config.public_url || undefined,
      },
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
      type: 'CloudflareD1',
      details: {
        account_id: d1Config.account_id,
        database_id: d1Config.database_id,
        api_token: d1Config.api_token,
      },
    }
  }
}
</script>

<template>
  <div class="config-container">
    <div class="config-header">
      <div class="config-title-row">
        <div class="config-header-left">
          <a-button
            type="text"
            class="back-button"
            @click="handleBack"
            :title="t('common.back' as any) || 'Back'"
          >
            <template #icon><HomeOutlined /></template>
          </a-button>
          <span class="config-title-text">{{ t('config.title') }}</span>
        </div>

        <div class="config-header-actions sm-only">
          <a-button
            type="text"
            @click="handleImport"
            :loading="isImporting"
            :title="t('config.importConfig')"
          >
            <template #icon><UploadOutlined /></template>
            <span class="hidden sm:inline">{{ t('config.importConfig') }}</span>
          </a-button>
          <a-button
            type="text"
            @click="handleExport"
            :loading="isExporting"
            :title="t('config.exportConfig')"
          >
            <template #icon><DownloadOutlined /></template>
            <span class="hidden sm:inline">{{ t('config.exportConfig') }}</span>
          </a-button>
          <a-button type="primary" size="small" @click="handleSave" :loading="isSaving">
            {{ t('config.saveConfig') }}
          </a-button>
        </div>
      </div>

      <!-- Mobile Actions Row -->
      <div class="mobile-actions-row">
        <a-button type="text" size="small" @click="handleImport" :loading="isImporting">
          <template #icon><UploadOutlined /></template>
          <span>{{ t('config.importConfig') }}</span>
        </a-button>
        <a-button type="text" size="small" @click="handleExport" :loading="isExporting">
          <template #icon><DownloadOutlined /></template>
          <span>{{ t('config.exportConfig') }}</span>
        </a-button>
        <a-button type="primary" size="small" @click="handleSave" :loading="isSaving">
          {{ t('config.saveConfig') }}
        </a-button>
      </div>

      <a-menu v-model:selectedKeys="activeTab" mode="horizontal" class="config-menu">
        <a-menu-item key="system">
          <template #icon><SettingOutlined /></template>
          <span>{{ t('config.categorySystem') }}</span>
        </a-menu-item>
        <a-menu-item key="books">
          <template #icon><BookOutlined /></template>
          <span>{{ t('config.categoryBooks') }}</span>
        </a-menu-item>
        <a-menu-item key="database">
          <template #icon><DatabaseOutlined /></template>
          <span>{{ t('config.categoryDatabase') }}</span>
        </a-menu-item>
      </a-menu>
    </div>

    <div class="config-content">
      <div
        v-if="showOperationDiagnostics && operationDiagnostics.length > 0"
        class="operation-diagnostics"
      >
        <div class="operation-diagnostics-header">
          <span class="operation-diagnostics-title">导入/保存诊断</span>
          <div class="operation-diagnostics-actions">
            <a-button type="link" size="small" @click="copyOperationDiagnostics">复制</a-button>
            <a-button type="link" size="small" @click="clearOperationDiagnostics">清空</a-button>
          </div>
        </div>
        <pre class="operation-diagnostics-body">{{ operationDiagnostics.join('\n') }}</pre>
      </div>

      <div class="tab-container">
        <!-- System Configuration -->
        <SystemSettings
          v-if="currentTab === 'system'"
          v-model:language="language"
          v-model:themeMode="themeMode"
          v-model:logLevel="logLevel"
          v-model:enableDebugTools="enableDebugTools"
          v-model:enableAutoCheck="enableAutoCheck"
          v-model:checkIntervalMins="checkIntervalMins"
          :debug-features-available="debugFeaturesAvailable"
          :is-cloud-configured="isCloudConfigured"
        />

        <!-- Book Sources Configuration -->
        <BookSourceSettings
          v-else-if="currentTab === 'books'"
          v-model:sourceType="sourceType"
          v-model:localBookPath="localBookPath"
          :r2-config="r2Config"
          :is-testing="isTesting"
          @select-folder="selectBookFolder"
          @test-connection="testConnection"
        />

        <!-- Database Configuration -->
        <DatabaseSettings
          v-else-if="currentTab === 'database'"
          v-model:dbType="dbType"
          v-model:sqlitePath="sqlitePath"
          :d1-config="d1Config"
          :is-testing="isTesting"
          @choose-sqlite-path="selectSqliteDatabasePath"
          @restore-default-sqlite-path="restoreDefaultSqlitePath"
          @copy-path="copyToClipboard"
          @test-connection="testConnection"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: v-bind('token.colorBgContainer');
  width: 100%;
}

.config-header {
  border-bottom: 1px solid v-bind('token.colorBorderSecondary');
  background: v-bind('token.colorBgContainer');
  padding: 0 8px;
}

.config-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 48px;
}

.config-header-left {
  display: flex;
  align-items: center;
  gap: 2px;
}

.back-button {
  margin-left: -4px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Ensure icons and text are perfectly aligned in buttons */
:deep(.ant-btn) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

:deep(.ant-btn .anticon) {
  line-height: 0;
  vertical-align: middle;
  margin-top: -1px; /* Optical adjustment */
}

.config-title-text {
  font-size: 16px;
  font-weight: 600;
  color: v-bind('token.colorText');
  line-height: 1;
  display: flex;
  align-items: center;
}

.config-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mobile-actions-row {
  display: none;
  padding-bottom: 8px;
  gap: 8px;
  align-items: center;
  justify-content: flex-end;
}

.flex-grow {
  flex-grow: 1;
}

.config-menu {
  border-bottom: none;
  background: transparent;
  line-height: 40px;
  height: 40px;
  display: flex;
  justify-content: center;
}

@media (max-width: 640px) {
  .config-header {
    padding: 0 12px;
  }

  .sm-only {
    display: none;
  }

  .mobile-actions-row {
    display: flex;
    flex-wrap: wrap;
    border-top: 1px solid v-bind('token.colorBorderSecondary');
    padding-top: 8px;
    margin-top: 0;
  }

  .config-menu {
    margin-top: 4px;
    height: auto;
    line-height: normal;
    padding: 8px 0;
    border-top: 1px solid v-bind('token.colorBorderSecondary');
  }
}

.config-content {
  flex: 1;
  padding: 16px 8px;
  overflow-y: auto;
}

.operation-diagnostics {
  max-width: 800px;
  margin: 0 auto 12px;
  border: 1px solid v-bind('token.colorBorderSecondary');
  border-radius: 8px;
  background: v-bind('token.colorBgContainer');
  overflow: hidden;
}

.operation-diagnostics-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-bottom: 1px solid v-bind('token.colorBorderSecondary');
}

.operation-diagnostics-title {
  font-size: 12px;
  font-weight: 600;
  color: v-bind('token.colorTextSecondary');
}

.operation-diagnostics-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.operation-diagnostics-body {
  margin: 0;
  padding: 8px 10px;
  max-height: 140px;
  overflow-y: auto;
  font-size: 11px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
  color: v-bind('token.colorText');
  font-family:
    ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
    monospace;
}

@media (max-width: 640px) {
  .config-content {
    padding: 16px;
  }
}

.config-breadcrumb {
  margin-bottom: 24px;
}

.tab-container {
  max-width: 800px;
  margin: 0 auto;
  width: 100%;
}

.form-footer-actions {
  margin-top: 24px;
  padding-left: 25%; /* To align with form items when label span is 6 (25%) */
}

@media (max-width: 575px) {
  .form-footer-actions {
    padding-left: 0;
    display: flex;
    justify-content: center;
  }
}

.cursor-pointer {
  cursor: pointer;
}

.cursor-pointer:hover {
  color: v-bind('token.colorPrimary');
}
</style>
