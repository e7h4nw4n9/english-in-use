<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { AppConfig, BookSource, BookSourceType, DatabaseConnection, DatabaseType } from '../types';
import { useI18n } from 'vue-i18n';
import { useTheme } from '../composables/useTheme';
import { 
  SettingOutlined, 
  BookOutlined, 
  DatabaseOutlined,
  FolderOpenOutlined,
  FileSearchOutlined,
  ArrowLeftOutlined,
  DownloadOutlined,
  UploadOutlined
} from '@ant-design/icons-vue';
import { message as antMessage, theme, Modal } from 'ant-design-vue';

const { t, locale } = useI18n();
const { setTheme } = useTheme();
const { useToken } = theme;
const { token } = useToken();

const emit = defineEmits<{
  (e: 'config-saved', config: AppConfig): void;
  (e: 'back'): void;
}>();

const props = defineProps<{
  initialConfig?: AppConfig;
}>();

const activeTab = ref<string[]>(['system']);

const currentTab = computed(() => activeTab.value[0]);

// System Config
const language = ref(props.initialConfig?.system?.language || 'en');
const themeMode = ref(props.initialConfig?.system?.theme || 'system');

// Book Source Config
const sourceType = ref<BookSourceType>(props.initialConfig?.book_source?.type || 'Local');
const localBookPath = ref(props.initialConfig?.book_source?.type === 'Local' ? props.initialConfig.book_source.details.path : '');
const r2Config = reactive({
  account_id: props.initialConfig?.book_source?.type === 'CloudflareR2' ? props.initialConfig.book_source.details.account_id : '',
  bucket_name: props.initialConfig?.book_source?.type === 'CloudflareR2' ? props.initialConfig.book_source.details.bucket_name : '',
  access_key_id: props.initialConfig?.book_source?.type === 'CloudflareR2' ? props.initialConfig.book_source.details.access_key_id : '',
  secret_access_key: props.initialConfig?.book_source?.type === 'CloudflareR2' ? props.initialConfig.book_source.details.secret_access_key : '',
  public_url: props.initialConfig?.book_source?.type === 'CloudflareR2' ? props.initialConfig.book_source.details.public_url || '' : '',
});

// Database Config
const dbType = ref<DatabaseType>('PostgreSQL');
const pgConfig = reactive({
  host: props.initialConfig?.database?.type === 'PostgreSQL' ? props.initialConfig.database.details.host : 'localhost',
  port: props.initialConfig?.database?.type === 'PostgreSQL' ? props.initialConfig.database.details.port : 5432,
  user: props.initialConfig?.database?.type === 'PostgreSQL' ? props.initialConfig.database.details.user : '',
  password: props.initialConfig?.database?.type === 'PostgreSQL' ? props.initialConfig.database.details.password : '',
  database: props.initialConfig?.database?.type === 'PostgreSQL' ? props.initialConfig.database.details.database : '',
  ssl: props.initialConfig?.database?.type === 'PostgreSQL' ? props.initialConfig.database.details.ssl : false,
});

const isTesting = ref(false);
const isExporting = ref(false);
const isImporting = ref(false);
const isSaving = ref(false);

function updateFormFromConfig(config: AppConfig) {
  // Reset reactive state to ensure full overwrite
  localBookPath.value = '';
  Object.assign(r2Config, {
    account_id: '',
    bucket_name: '',
    access_key_id: '',
    secret_access_key: '',
    public_url: '',
  });
  Object.assign(pgConfig, {
    host: 'localhost',
    port: 5432,
    user: '',
    password: '',
    database: '',
    ssl: false,
  });

  // Update system config
  if (config.system) {
    language.value = config.system.language;
    themeMode.value = config.system.theme as 'system' | 'light' | 'dark';
  }

  // Update book source config
  if (config.book_source) {
    sourceType.value = config.book_source.type;
    if (config.book_source.type === 'Local') {
      localBookPath.value = config.book_source.details.path;
    } else if (config.book_source.type === 'CloudflareR2') {
      const details = config.book_source.details;
      r2Config.account_id = details.account_id;
      r2Config.bucket_name = details.bucket_name;
      r2Config.access_key_id = details.access_key_id;
      r2Config.secret_access_key = details.secret_access_key;
      r2Config.public_url = details.public_url || '';
    }
  } else {
    sourceType.value = 'Local';
  }

  // Update database config
  if (config.database) {
    dbType.value = config.database.type;
    if (config.database.type === 'PostgreSQL') {
      const details = config.database.details;
      pgConfig.host = details.host;
      pgConfig.port = details.port;
      pgConfig.user = details.user;
      pgConfig.password = details.password || '';
      pgConfig.database = details.database;
      pgConfig.ssl = details.ssl;
    }
  } else {
    dbType.value = 'PostgreSQL';
  }
}

async function handleSave() {
  isSaving.value = true;
  try {
    const config: AppConfig = {
      system: {
        language: language.value,
        theme: themeMode.value as 'system' | 'light' | 'dark',
      },
      book_source: getCurrentBookSource(),
      database: getCurrentDatabase(),
    };

    await invoke('save_config', { config });
    
    locale.value = language.value;
    setTheme(themeMode.value as any);
    
    Modal.success({
      title: t('config.savedSuccess'),
      content: t('config.restartNotice' as any),
      okText: t('common.ok' as any) || 'OK',
      onOk: () => {
        invoke('restart');
      },
    });
  } catch (err) {
    antMessage.error(t('config.saveError', { error: err }));
  } finally {
    isSaving.value = false;
  }
}

async function handleExport() {
  isExporting.value = true;
  try {
    const config: AppConfig = {
      system: {
        language: language.value,
        theme: themeMode.value as 'system' | 'light' | 'dark',
      },
      book_source: getCurrentBookSource(),
      database: getCurrentDatabase(),
    };

    let filePath = await save({
      filters: [
        {
          name: 'TOML Configuration',
          extensions: ['toml'],
        },
      ],
      defaultPath: 'english-in-use-config.toml',
    });

    if (filePath) {
      // Ensure the file has .toml extension
      if (!filePath.toLowerCase().endsWith('.toml')) {
        filePath += '.toml';
      }
      await invoke('export_config', { path: filePath, config });
      antMessage.success(t('config.exportSuccess'));
    }
  } catch (err) {
    antMessage.error(t('config.exportError', { error: err }));
  } finally {
    isExporting.value = false;
  }
}

async function handleImport() {
  isImporting.value = true;
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'TOML Configuration',
          extensions: ['toml'],
        },
      ],
    });

    if (selected && typeof selected === 'string') {
      const config: AppConfig = await invoke('import_config', { path: selected });
      
      // Save to application config immediately (overwrite current config.toml)
      await invoke('save_config', { config });
      
      // Update form fields
      updateFormFromConfig(config);
      
      // Apply system settings immediately
      locale.value = language.value;
      setTheme(themeMode.value as any);
      
      Modal.success({
        title: t('config.importSuccess'),
        content: t('config.restartNotice' as any),
        okText: t('common.ok' as any) || 'OK',
        onOk: () => {
          invoke('restart');
        },
      });
    }
  } catch (err) {
    antMessage.error(t('config.importError', { error: err }));
  } finally {
    isImporting.value = false;
  }
}

async function testConnection() {
  isTesting.value = true;
  try {
    if (currentTab.value === 'books') {
      const source = getCurrentBookSource();
      if (source?.type !== 'CloudflareR2') return;
      await invoke('test_r2_connection', { source });
      antMessage.success(t('config.testSuccess'));
    } else if (currentTab.value === 'database') {
      const connection = getCurrentDatabase();
      if (!connection) return;
      await invoke('test_postgresql_connection', { connection });
      antMessage.success(t('config.testSuccess'));
    }
  } catch (err) {
    antMessage.error(t('config.connectionFailed', { error: err }));
  } finally {
    isTesting.value = false;
  }
}

async function selectBookFolder() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: localBookPath.value || undefined,
    });
    if (selected && typeof selected === 'string') {
      localBookPath.value = selected;
    }
  } catch (err) {
    console.error('Failed to open dialog:', err);
  }
}

function getCurrentBookSource(): BookSource | null {
  if (sourceType.value === 'Local') {
    return {
      type: 'Local',
      details: { path: localBookPath.value },
    };
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
    };
  }
}

function getCurrentDatabase(): DatabaseConnection | null {
  return {
    type: 'PostgreSQL',
    details: {
      host: pgConfig.host,
      port: Number(pgConfig.port),
      user: pgConfig.user,
      password: pgConfig.password || undefined,
      database: pgConfig.database,
      ssl: pgConfig.ssl,
    },
  };
}
</script>

<template>
  <div class="config-container">
    <div class="config-header">
      <div class="config-title-row">
        <div class="config-header-left">
          <a-button type="text" @click="emit('back')" class="back-button">
            <template #icon><ArrowLeftOutlined /></template>
          </a-button>
          <span class="config-title-text">{{ t('config.title') }}</span>
        </div>

        <div class="config-header-actions sm-only">
          <a-button type="text" @click="handleImport" :loading="isImporting" :title="t('config.importConfig')">
            <template #icon><UploadOutlined /></template>
            <span class="hidden sm:inline">{{ t('config.importConfig') }}</span>
          </a-button>
          <a-button type="text" @click="handleExport" :loading="isExporting" :title="t('config.exportConfig')">
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
        <a-form v-if="currentTab === 'system'" layout="horizontal" :label-col="{ xs: { span: 24 }, sm: { span: 6 } }" :wrapper-col="{ xs: { span: 24 }, sm: { span: 18 } }" label-align="right">
          <a-form-item :label="t('config.language')">
            <a-select v-model:value="language">
              <a-select-option value="en">English</a-select-option>
              <a-select-option value="zh">中文</a-select-option>
            </a-select>
          </a-form-item>
          <a-form-item :label="t('config.theme')">
            <a-select v-model:value="themeMode">
              <a-select-option value="system">{{ t('config.themeSystem') }}</a-select-option>
              <a-select-option value="light">{{ t('config.themeLight') }}</a-select-option>
              <a-select-option value="dark">{{ t('config.themeDark') }}</a-select-option>
            </a-select>
          </a-form-item>
        </a-form>

        <!-- Book Sources Configuration -->
        <div v-if="currentTab === 'books'">
          <a-form layout="horizontal" :label-col="{ xs: { span: 24 }, sm: { span: 6 } }" :wrapper-col="{ xs: { span: 24 }, sm: { span: 18 } }" label-align="right">
            <a-form-item :label="t('config.sourceType')">
              <a-radio-group v-model:value="sourceType" button-style="solid">
                <a-radio-button value="Local">{{ t('config.localFolder') }}</a-radio-button>
                <a-radio-button value="CloudflareR2">{{ t('config.cloudR2') }}</a-radio-button>
              </a-radio-group>
            </a-form-item>

            <div v-if="sourceType === 'Local'">
              <a-form-item :label="t('config.folderPath')">
                <a-input v-model:value="localBookPath" :placeholder="t('config.folderPath')">
                  <template #addonAfter>
                    <FolderOpenOutlined @click="selectBookFolder" class="cursor-pointer" />
                  </template>
                </a-input>
              </a-form-item>
            </div>

            <div v-else>
              <a-form-item :label="t('config.accountId')">
                <a-input v-model:value="r2Config.account_id" />
              </a-form-item>
              <a-form-item :label="t('config.bucketName')">
                <a-input v-model:value="r2Config.bucket_name" />
              </a-form-item>
              <a-form-item :label="t('config.accessKeyId')">
                <a-input v-model:value="r2Config.access_key_id" />
              </a-form-item>
              <a-form-item :label="t('config.secretAccessKey')">
                <a-input-password v-model:value="r2Config.secret_access_key" />
              </a-form-item>
              <a-form-item :label="t('config.publicUrl')">
                <a-input v-model:value="r2Config.public_url" placeholder="https://..." />
              </a-form-item>
            </div>
          </a-form>
          
          <div v-if="sourceType === 'CloudflareR2'" class="form-footer-actions">
            <a-button @click="testConnection" :loading="isTesting">
              {{ t('config.testConnection') }}
            </a-button>
          </div>
        </div>

        <!-- Database Configuration -->
        <div v-if="currentTab === 'database'">
          <a-form layout="horizontal" :label-col="{ xs: { span: 24 }, sm: { span: 6 } }" :wrapper-col="{ xs: { span: 24 }, sm: { span: 18 } }" label-align="right">
            <a-form-item label="Host">
              <a-input v-model:value="pgConfig.host" placeholder="localhost" />
            </a-form-item>
            <a-form-item label="Port">
              <a-input-number v-model:value="pgConfig.port" :min="1" :max="65535" style="width: 100%" />
            </a-form-item>
            <a-form-item label="User">
              <a-input v-model:value="pgConfig.user" />
            </a-form-item>
            <a-form-item label="Password">
              <a-input-password v-model:value="pgConfig.password" />
            </a-form-item>
            <a-form-item label="Database">
              <a-input v-model:value="pgConfig.database" />
            </a-form-item>
            <a-form-item label="SSL">
              <a-switch v-model:checked="pgConfig.ssl" />
            </a-form-item>
          </a-form>

          <div class="form-footer-actions">
            <a-button @click="testConnection" :loading="isTesting">
              {{ t('config.testConnection') }}
            </a-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-container {
  height: calc(100vh - 28px);
  display: flex;
  flex-direction: column;
  background: v-bind('token.colorBgContainer');
}

.config-header {
  border-bottom: 1px solid v-bind('token.colorBorderSecondary');
  background: v-bind('token.colorBgContainer');
  padding: 0 24px;
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
  gap: 8px;
}

.back-button {
  margin-left: -8px;
}

.config-title-text {
  font-size: 16px;
  font-weight: 600;
  color: v-bind('token.colorText');
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
  padding: 24px;
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
  max-width: 600px;
  margin: 0 auto;
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