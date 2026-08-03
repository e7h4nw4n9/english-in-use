import { watch } from 'vue'
import { warn } from '@tauri-apps/plugin-log'
import { message as antMessage } from 'ant-design-vue'
import {
  getDefaultSqlitePath,
  resolveSqlitePath as resolveSqlitePathFromApi,
  validateLocalBookSource,
} from '../../lib/api'
import type { AppConfig } from '../../types'
import type { ConfigPageContext } from './configPageContext'

/**
 * 管理 SQLite 路径规范化、云同步目录限制和本地目录校验。
 * @param context - 配置页面共享状态。
 */
export function useConfigPathOperations(context: ConfigPageContext) {
  const {
    initialConfig,
    t,
    appStore,
    dbType,
    sqlitePath,
    appendOperationDiagnostic,
    withTimeout,
    sanitizeErrorMessage,
    registerPathHooks,
  } = context

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

  /** 判断路径是否位于已知云同步目录。
   * @param path - 待检查路径。
   */
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
      // Windows 文件 URL 可能采用 /C:/foo/bar 形式。
      if (/^\/[a-zA-Z]:\//.test(decoded)) {
        return decoded.slice(1)
      }
      return decoded
    } catch {
      return trimmed
    }
  }

  /** 获取本地默认 SQLite 路径，并记录触发回退的原因。
   * @param reason - 触发路径回退的业务原因。
   */
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

  /** 阻止使用云同步 SQLite 路径，并按场景切换到安全本地路径。
   * @param path - 用户当前选择的 SQLite 路径。
   * @param reason - 执行策略检查的业务场景。
   */
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

  async function ensureConfigSqlitePathNotCloud(
    config: AppConfig,
    reason: string,
  ): Promise<boolean> {
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

  watch(dbType, (newType) => {
    if (newType === 'SQLite') {
      if (!sqlitePath.value) {
        fetchDefaultSqlitePath()
        return
      }
      void ensureCurrentSqlitePathNotCloud('切换到 SQLite')
    }
  })

  // SQLite 模式下首次加载默认路径。
  if (dbType.value === 'SQLite') {
    if (initialConfig?.database?.type === 'SQLite') {
      sqlitePath.value = String(initialConfig.database.details.path || '').trim()
    } else {
      fetchDefaultSqlitePath()
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

  /** 解析并校验 SQLite 路径，同时执行云同步目录限制。
   * @param config - 待校验且允许规范化路径的配置。
   */
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

  /** 校验本地图书源目录及其必要结构。
   * @param config - 待保存或导入的配置。
   */
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

  registerPathHooks({
    fetchDefaultSqlitePath: () => {
      void fetchDefaultSqlitePath()
    },
    ensureCurrentSqlitePathNotCloud: (reason) => {
      void ensureCurrentSqlitePathNotCloud(reason)
    },
  })
  if (dbType.value === 'SQLite' && sqlitePath.value.trim()) {
    void ensureCurrentSqlitePathNotCloud('加载现有配置')
  }

  return {
    normalizeSqlitePath,
    isAbsolutePath,
    isAppleMobileDevice,
    normalizeDialogSelectedPath,
    enforceSqliteCloudPathPolicy,
    ensureConfigSqlitePathNotCloud,
    ensureCurrentSqlitePathNotCloud,
    resolveSqlitePathForUsage,
    fetchDefaultSqlitePath,
    ensureSqlitePathForImportIfNeeded,
    validateSqlitePathIfNeeded,
    validateLocalBookSourceIfNeeded,
  }
}

export type ConfigPathOperations = ReturnType<typeof useConfigPathOperations>
