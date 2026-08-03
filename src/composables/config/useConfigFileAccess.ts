import { open, save } from '@tauri-apps/plugin-dialog'
import { warn } from '@tauri-apps/plugin-log'
import { message as antMessage } from 'ant-design-vue'
import { getDefaultSqlitePath } from '../../lib/api'
import type { AppConfig } from '../../types'
import type { ConfigPageContext } from './configPageContext'
import type { ConfigPathOperations } from './useConfigPathOperations'

/**
 * 管理配置页面的文件选择和 Apple 移动端沙盒授权。
 * @param context - 配置页面共享状态。
 * @param paths - SQLite 路径策略操作。
 */
export function useConfigFileAccess(context: ConfigPageContext, paths: ConfigPathOperations) {
  const { localBookPath, sqlitePath, appendOperationDiagnostic, sanitizeErrorMessage } = context
  const {
    normalizeSqlitePath,
    isAppleMobileDevice,
    normalizeDialogSelectedPath,
    enforceSqliteCloudPathPolicy,
    ensureConfigSqlitePathNotCloud,
    resolveSqlitePathForUsage,
    fetchDefaultSqlitePath,
  } = paths

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

  /** 在 Apple 移动端通过临时标记文件获得目录授权。
   * @param title - 文件选择器标题。
   * @param prefix - 临时标记文件名前缀。
   * @param defaultPath - 默认打开路径。
   */
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

  /** 在 Apple 移动端选择或创建 SQLite 文件路径。
   * @param currentPath - 当前表单中的 SQLite 路径。
   */
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

  /** 将目录、文件 URL 或普通路径解析为可供后端使用的 SQLite 文件路径。
   * @param rawPath - 用户输入或文件选择器返回的路径。
   */

  /** 在 Apple 移动端确认本地图书目录仍具备沙盒访问权限。
   * @param config - 待保存或导入的配置。
   */
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

  return {
    ensureAppleMobileLocalAuthorizationIfNeeded,
    selectBookFolder,
    selectSqliteDatabasePath,
    restoreDefaultSqlitePath,
  }
}

export type ConfigFileAccess = ReturnType<typeof useConfigFileAccess>
