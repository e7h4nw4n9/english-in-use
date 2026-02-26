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
import {
  SettingOutlined,
  BookOutlined,
  DatabaseOutlined,
  ArrowLeftOutlined,
  HomeOutlined,
  DownloadOutlined,
  UploadOutlined,
} from '@ant-design/icons-vue'
import { message as antMessage, theme, Modal } from 'ant-design-vue'
import {
  saveConfig,
  exportConfig,
  importConfig,
  getDefaultSqlitePath,
  testDatabaseConnection,
  testR2Connection,
} from '../lib/api'
import { restartApp } from '../lib/api/system'
import SystemSettings from './config/SystemSettings.vue'
import BookSourceSettings from './config/BookSourceSettings.vue'
import DatabaseSettings from './config/DatabaseSettings.vue'

const { t, locale } = useI18n()
const { setTheme } = useTheme()
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
  (e: 'back'): void
}>()

const activeTab = ref<string[]>(['system'])

const currentTab = computed(() => activeTab.value[0])

// System Config
const language = ref(props.initialConfig?.system?.language || 'en')
const themeMode = ref(props.initialConfig?.system?.theme || 'system')
const logLevel = ref(props.initialConfig?.system?.log_level || 'info')
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

async function fetchDefaultSqlitePath() {
  try {
    const path = await getDefaultSqlitePath()
    sqlitePath.value = path
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
  if (newType === 'SQLite' && !sqlitePath.value) {
    fetchDefaultSqlitePath()
  }
})

// Initial fetch if type is SQLite
if (dbType.value === 'SQLite') {
  if (props.initialConfig?.database?.type === 'SQLite') {
    sqlitePath.value = props.initialConfig.database.details.path
  } else {
    fetchDefaultSqlitePath()
  }
}

const isTesting = ref(false)
const isExporting = ref(false)
const isImporting = ref(false)
const isSaving = ref(false)

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
      sqlitePath.value = config.database.details.path
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

async function handleSave() {
  isSaving.value = true
  info('用户触发了保存配置操作')
  try {
    const config: AppConfig = {
      system: {
        language: language.value,
        theme: themeMode.value as 'system' | 'light' | 'dark',
        log_level: logLevel.value as any,
        enable_auto_check: enableAutoCheck.value,
        check_interval_mins: checkIntervalMins.value,
      },
      book_source: getCurrentBookSource(),
      database: getCurrentDatabase(),
    }

    await saveConfig(config)
    info('配置保存成功')

    locale.value = language.value
    setTheme(themeMode.value as any)

    Modal.success({
      title: t('config.savedSuccess'),
      content: t('config.restartNotice' as any),
      okText: t('common.ok' as any) || 'OK',
      onOk: () => {
        info('用户点击重启应用')
        restartApp()
      },
    })
  } catch (err) {
    error(`保存配置失败: ${err}`)
    antMessage.error(t('config.saveError', { error: err }))
  } finally {
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
  info('用户触发了导入配置操作')
  try {
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
      debug(`选择导入的文件: ${selected}`)
      const config: AppConfig = await importConfig(selected)

      // Save to application config immediately (overwrite current config.toml)
      await saveConfig(config)
      info('配置文件导入并保存成功')

      // Update form fields
      updateFormFromConfig(config)

      // Apply system settings immediately
      locale.value = language.value
      setTheme(themeMode.value as any)

      Modal.success({
        title: t('config.importSuccess'),
        content: t('config.restartNotice' as any),
        okText: t('common.ok' as any) || 'OK',
        onOk: () => {
          info('用户点击重启应用 (导入后)')
          restartApp()
        },
      })
    } else {
      debug('用户取消了导入操作')
    }
  } catch (err) {
    error(`导入配置失败: ${err}`)
    antMessage.error(t('config.importError', { error: err }))
  } finally {
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
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: localBookPath.value || undefined,
    })
    if (selected && typeof selected === 'string') {
      localBookPath.value = selected
    }
  } catch (err) {
    console.error('Failed to open dialog:', err)
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
        path: sqlitePath.value,
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
            @click="emit('back')"
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
      <div class="tab-container">
        <!-- System Configuration -->
        <SystemSettings
          v-if="currentTab === 'system'"
          v-model:language="language"
          v-model:themeMode="themeMode"
          v-model:logLevel="logLevel"
          v-model:enableAutoCheck="enableAutoCheck"
          v-model:checkIntervalMins="checkIntervalMins"
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
          :sqlite-path="sqlitePath"
          :d1-config="d1Config"
          :is-testing="isTesting"
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
