import { open, save } from '@tauri-apps/plugin-dialog'
import { debug, error, info, warn } from '@tauri-apps/plugin-log'
import { message as antMessage, Modal } from 'ant-design-vue'
import {
  exportConfig,
  getDefaultSqlitePath,
  importConfig,
  initializeDatabase,
  saveConfig,
  testCloudflareGateway,
  testDatabaseConnection,
} from '../../lib/api'
import type { AppConfig } from '../../types'
import type { ConfigFileAccess } from './useConfigFileAccess'
import type { ConfigPageContext } from './configPageContext'
import type { ConfigPathOperations } from './useConfigPathOperations'

/**
 * 编排配置保存、导入导出、连接检查和数据库初始化。
 * @param context - 配置页面共享状态。
 * @param paths - 路径策略与文件授权操作。
 */
export function useConfigPersistenceOperations(
  context: ConfigPageContext,
  paths: ConfigPathOperations & ConfigFileAccess,
) {
  const {
    t,
    locale,
    setTheme,
    appStore,
    sqlitePath,
    language,
    themeMode,
    handleBack,
    currentTab,
    isTesting,
    isExporting,
    isImporting,
    isSaving,
    shouldReloadHomeAfterConfigChange,
    appendOperationDiagnostic,
    resetOperationDiagnostics,
    sanitizeErrorMessage,
    withTimeout,
    buildConfigFromForm,
    updateFormFromConfig,
    getCurrentBookSource,
    getCurrentDatabase,
    gatewayConfig,
    gatewayConfigurationRequired,
    emit,
  } = context
  const {
    normalizeSqlitePath,
    isAbsolutePath,
    isAppleMobileDevice,
    resolveSqlitePathForUsage,
    enforceSqliteCloudPathPolicy,
    ensureSqlitePathForImportIfNeeded,
    validateSqlitePathIfNeeded,
    validateLocalBookSourceIfNeeded,
    ensureAppleMobileLocalAuthorizationIfNeeded,
  } = paths

  async function checkCloudConnections(
    config: AppConfig,
  ): Promise<{ isD1Connected: boolean; hasWarnings: boolean }> {
    appendOperationDiagnostic('开始检查云连接')
    appStore.setGlobalLoadingMessage(t('config.checkingConnections' as any) || '正在检查云连接...')
    appStore.setGlobalLoadingProgress(45)
    const usesGateway =
      config.book_source?.type === 'CloudflareGateway' ||
      config.database?.type === 'CloudflareGateway'
    let isD1Connected = config.database?.type !== 'CloudflareGateway'
    const needsR2 = config.book_source?.type === 'CloudflareGateway'
    const needsD1 = config.database?.type === 'CloudflareGateway'
    let hasWarnings = false
    const connectionTimeoutMs = 20_000

    if (usesGateway && config.cloudflare_gateway) {
      appStore.setGlobalLoadingMessage('正在检查 Cloudflare 网关...')
      appStore.setGlobalLoadingProgress(55)
      appendOperationDiagnostic('开始检查 Cloudflare 网关')
      info('开始检查 Cloudflare 网关')
      try {
        const status = await withTimeout(
          testCloudflareGateway(config.cloudflare_gateway),
          connectionTimeoutMs,
          'Cloudflare 网关连接检查',
        )
        const isR2Connected = !needsR2 || status.r2.status === 'Connected'
        isD1Connected = !needsD1 || status.database.status === 'Connected'
        if (!isR2Connected || !isD1Connected) {
          const unavailableServices = [
            !isR2Connected ? 'R2' : '',
            !isD1Connected ? 'D1' : '',
          ].filter(Boolean)
          throw new Error(`${unavailableServices.join('、')} 绑定不可用`)
        }
        appendOperationDiagnostic('Cloudflare 网关连接检查完成')
        info('Cloudflare 网关连接检查完成')
      } catch (err) {
        hasWarnings = true
        const errMsg = sanitizeErrorMessage(err)
        if (config.database?.type === 'CloudflareGateway') isD1Connected = false
        appendOperationDiagnostic(`Cloudflare 网关检查失败（忽略并继续）: ${errMsg}`)
        warn(`Cloudflare 网关检查失败（不影响配置保存/导入）: ${errMsg}`)
        antMessage.warning(
          t('config.connectionCheckWarning', { service: 'Cloudflare', error: errMsg }),
        )
      }
    }

    appendOperationDiagnostic(`云连接检查结束，D1可用: ${isD1Connected ? '是' : '否'}`)
    return { isD1Connected, hasWarnings }
  }

  /** 在连接可用时初始化数据库，并将失败降级为可见警告。
   * @param config - 已保存的应用配置。
   * @param isD1Connected - D1 连接检查结果。
   */
  async function initializeDatabaseIfNeeded(
    config: AppConfig,
    isD1Connected: boolean,
  ): Promise<boolean> {
    const databaseType = config.database?.type
    if (!databaseType) {
      appendOperationDiagnostic('跳过数据库初始化（未配置数据库）')
      return true
    }

    if (databaseType === 'CloudflareGateway' && !isD1Connected) {
      appendOperationDiagnostic('跳过数据库初始化（D1 不可用）')
      return false
    }

    try {
      appStore.setGlobalLoadingMessage(
        t('config.initializingDatabase' as any) || '正在初始化数据库...',
      )
      appStore.setGlobalLoadingProgress(80)
      appendOperationDiagnostic(`开始初始化数据库: ${databaseType}`)
      await initializeDatabase()
      appendOperationDiagnostic(`数据库初始化完成: ${databaseType}`)
      info(`${databaseType} 数据库初始化完成`)
      return true
    } catch (err) {
      const errMsg = sanitizeErrorMessage(err)
      if (
        databaseType === 'SQLite' &&
        isAppleMobileDevice() &&
        /operation not permitted|permission denied|os error 1/i.test(String(err))
      ) {
        appendOperationDiagnostic('检测到 iPadOS SQLite 权限错误，尝试切换默认路径并重试初始化')
        try {
          const defaultPath = await withTimeout(
            getDefaultSqlitePath(),
            8_000,
            '获取默认 SQLite 路径',
          )
          const normalizedDefaultPath = normalizeSqlitePath(defaultPath || '')
          if (normalizedDefaultPath && config.database?.type === 'SQLite') {
            config.database.details.path = normalizedDefaultPath
            sqlitePath.value = normalizedDefaultPath
            await saveConfig(config)
            await initializeDatabase()
            appendOperationDiagnostic('SQLite 默认路径重试初始化成功')
            antMessage.warning('iPadOS 本地目录权限受限，已自动切换到应用默认数据库路径')
            return true
          }
        } catch (retryErr) {
          appendOperationDiagnostic(`SQLite 默认路径重试失败: ${sanitizeErrorMessage(retryErr)}`)
        }
      }

      appendOperationDiagnostic(`数据库初始化失败（忽略并继续）: ${errMsg}`)
      warn(`数据库初始化失败（不影响配置保存/导入）: ${errMsg}`)
      antMessage.warning(t('config.databaseInitWarning', { error: errMsg }))
      return false
    }
  }

  async function handleSave() {
    isSaving.value = true
    resetOperationDiagnostics('保存配置')
    info('用户触发了保存配置操作')
    try {
      appStore.startGlobalLoading(t('config.savingConfig' as any) || '正在保存配置...')
      appStore.setGlobalLoadingProgress(10)

      const config = buildConfigFromForm()

      const appleAuthorized = await ensureAppleMobileLocalAuthorizationIfNeeded(config)
      if (!appleAuthorized) return

      const localSourceValid = await validateLocalBookSourceIfNeeded(config)
      if (!localSourceValid) return

      const sqlitePathValid = await validateSqlitePathIfNeeded(config)
      if (!sqlitePathValid) return

      appendOperationDiagnostic('开始写入配置文件')
      await saveConfig(config)
      gatewayConfigurationRequired.value = false
      appStore.setGlobalLoadingProgress(35)
      appendOperationDiagnostic('配置文件写入完成')
      info('配置保存成功')

      const { isD1Connected, hasWarnings } = await checkCloudConnections(config)
      const databaseReady = await initializeDatabaseIfNeeded(config, isD1Connected)
      appStore.setGlobalLoadingProgress(95)

      locale.value = language.value
      setTheme(themeMode.value as any)

      shouldReloadHomeAfterConfigChange.value = true
      emit('config-saved', config)

      if (hasWarnings || !databaseReady) {
        antMessage.warning('配置已保存，但部分连接或数据库初始化尚未就绪')
        return
      }

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
      const config = buildConfigFromForm()
      const includeSecrets = await new Promise<boolean>((resolve) => {
        Modal.confirm({
          title: '导出配置',
          content: '是否在导出文件中包含网关访问令牌？选择“脱敏导出”将清空令牌字段。',
          okText: '包含密钥',
          cancelText: '脱敏导出',
          onOk: () => resolve(true),
          onCancel: () => resolve(false),
        })
      })

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
        // 确保导出文件使用 .toml 扩展名。
        if (!filePath.toLowerCase().endsWith('.toml')) {
          filePath += '.toml'
        }
        await exportConfig(filePath, config, includeSecrets)
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

  /** 执行配置导入、路径授权、保存、连接检查和数据库初始化。 */
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
        config.system.auto_start_study_timer = config.system.auto_start_study_timer ?? false
        appStore.setGlobalLoadingProgress(25)
        appendOperationDiagnostic('导入配置读取完成')

        const selectsGateway =
          config.book_source?.type === 'CloudflareGateway' ||
          config.database?.type === 'CloudflareGateway'
        const importedGatewayIncomplete =
          selectsGateway &&
          (!config.cloudflare_gateway?.base_url.trim() ||
            !config.cloudflare_gateway?.access_token.trim())
        if (config.gateway_configuration_required || importedGatewayIncomplete) {
          config.gateway_configuration_required = true
          updateFormFromConfig(config)
          gatewayConfigurationRequired.value = true
          locale.value = language.value
          setTheme(themeMode.value as any)
          appendOperationDiagnostic('导入配置缺少网关授权，等待用户补充后保存')
          antMessage.warning(t('config.gatewayMigrationRequired' as any))
          return
        }

        const appleAuthorized = await ensureAppleMobileLocalAuthorizationIfNeeded(config)
        if (!appleAuthorized) return

        const localSourceValid = await validateLocalBookSourceIfNeeded(config)
        if (!localSourceValid) return
        const sqlitePathPrepared = await ensureSqlitePathForImportIfNeeded(config)
        if (!sqlitePathPrepared) return
        const sqlitePathValid = await validateSqlitePathIfNeeded(config)
        if (!sqlitePathValid) return

        // 立即覆盖应用配置文件，后续连接失败只作为警告。
        appendOperationDiagnostic('开始写入应用配置')
        await saveConfig(config)
        appStore.setGlobalLoadingProgress(40)
        appendOperationDiagnostic('应用配置写入完成')
        info('配置文件导入并保存成功')

        const { isD1Connected, hasWarnings } = await checkCloudConnections(config)
        const databaseReady = await initializeDatabaseIfNeeded(config, isD1Connected)
        appStore.setGlobalLoadingProgress(95)

        // 使用导入结果更新表单。
        updateFormFromConfig(config)

        // 立即应用语言和主题设置。
        locale.value = language.value
        setTheme(themeMode.value as any)

        shouldReloadHomeAfterConfigChange.value = true
        emit('config-imported')
        emit('config-saved', config)

        if (hasWarnings || !databaseReady) {
          antMessage.warning('配置已导入，但部分连接或数据库初始化尚未就绪')
          return
        }

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
        if (!source || source.type !== 'CloudflareGateway') {
          warn('尝试测试非网关类型的图书源连接')
          return
        }
        const status = await testCloudflareGateway({ ...gatewayConfig })
        if (status.r2.status !== 'Connected') {
          throw new Error(status.r2.status === 'Disconnected' ? status.r2.message : 'R2 绑定不可用')
        }
        info('Cloudflare 网关连接测试成功 (前端反馈)')
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
        if (connection.type === 'CloudflareGateway') {
          const status = await testCloudflareGateway({ ...gatewayConfig })
          if (status.database.status !== 'Connected') {
            throw new Error(
              status.database.status === 'Disconnected' ? status.database.message : 'D1 绑定不可用',
            )
          }
        } else {
          await testDatabaseConnection(connection)
        }
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

  return { handleSave, handleExport, handleImport, testConnection }
}
