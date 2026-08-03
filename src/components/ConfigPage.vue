<script setup lang="ts">
import type { AppConfig } from '../types'
import {
  SettingOutlined,
  BookOutlined,
  DatabaseOutlined,
  HomeOutlined,
  DownloadOutlined,
  UploadOutlined,
} from '@ant-design/icons-vue'
import SystemSettings from './config/SystemSettings.vue'
import BookSourceSettings from './config/BookSourceSettings.vue'
import DatabaseSettings from './config/DatabaseSettings.vue'
import { useConfigPage } from '../composables/config/useConfigPage'

const props = withDefaults(defineProps<{ initialConfig?: AppConfig; allowBack?: boolean }>(), {
  allowBack: true,
})
const emit = defineEmits<{
  (e: 'config-saved', config: AppConfig): void
  (e: 'config-imported'): void
  (e: 'back', options?: { reloadHome?: boolean }): void
}>()

const {
  t,
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
  showOperationDiagnostics,
  operationDiagnostics,
  handleBack,
  clearOperationDiagnostics,
  copyOperationDiagnostics,
  handleSave,
  handleExport,
  handleImport,
  selectBookFolder,
  testConnection,
  selectSqliteDatabasePath,
  restoreDefaultSqlitePath,
  copyToClipboard,
} = useConfigPage(props.initialConfig, props.allowBack, emit)
</script>

<template>
  <div class="config-container">
    <div class="config-header">
      <div class="config-title-row">
        <div class="config-header-left">
          <a-button
            v-if="showBackButton"
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

      <!-- 移动端操作栏 -->
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
      <a-alert
        v-if="gatewayConfigurationRequired"
        type="warning"
        show-icon
        class="gateway-migration-alert"
        :message="t('config.gatewayMigrationRequired' as any)"
      />
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
        <!-- 系统配置 -->
        <SystemSettings
          v-if="currentTab === 'system'"
          v-model:language="language"
          v-model:themeMode="themeMode"
          v-model:logLevel="logLevel"
          v-model:enableDebugTools="enableDebugTools"
          v-model:autoStartStudyTimer="autoStartStudyTimer"
          v-model:enableAutoCheck="enableAutoCheck"
          v-model:checkIntervalMins="checkIntervalMins"
          :debug-features-available="debugFeaturesAvailable"
          :is-cloud-configured="isCloudConfigured"
        />

        <!-- 图书来源配置 -->
        <BookSourceSettings
          v-else-if="currentTab === 'books'"
          v-model:sourceType="sourceType"
          v-model:localBookPath="localBookPath"
          :gateway-config="gatewayConfig"
          :is-testing="isTesting"
          @select-folder="selectBookFolder"
          @test-connection="testConnection"
        />

        <!-- 数据库配置 -->
        <DatabaseSettings
          v-else-if="currentTab === 'database'"
          v-model:dbType="dbType"
          v-model:sqlitePath="sqlitePath"
          :gateway-config="gatewayConfig"
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

/* 保持按钮中的图标与文字对齐。 */
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

.gateway-migration-alert {
  margin-bottom: 16px;
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
