<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { message } from 'ant-design-vue'
import {
  getMigrationVersions,
  getCurrentDbVersion,
  executeMigrationUp,
  executeMigrationDown,
} from '../../lib/api/database'

const { t } = useI18n()

interface Props {
  language: string
  themeMode: string
  logLevel: string
  enableAutoCheck: boolean
  checkIntervalMins: number
  isCloudConfigured: boolean
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:language', value: string): void
  (e: 'update:themeMode', value: string): void
  (e: 'update:logLevel', value: string): void
  (e: 'update:enableAutoCheck', value: boolean): void
  (e: 'update:checkIntervalMins', value: number): void
}>()

// Dev migration features
const isDev = import.meta.env.DEV
const currentVersion = ref<string>('')
const availableVersions = ref<string[]>([])
const selectedTargetVersion = ref<string | undefined>(undefined)
const migrationLoading = ref(false)

const loadMigrationInfo = async () => {
  if (!isDev) return
  try {
    const [version, versions] = await Promise.all([getCurrentDbVersion(), getMigrationVersions()])
    currentVersion.value = version
    availableVersions.value = versions
  } catch (error) {
    console.error('Failed to load migration info:', error)
  }
}

const handleUpgrade = async () => {
  migrationLoading.value = true
  try {
    await executeMigrationUp(selectedTargetVersion.value)
    message.success(t('config.migrationSuccess'))
    await loadMigrationInfo()
  } catch (error: any) {
    message.error(t('config.migrationError', { error: error.message || error }))
  } finally {
    migrationLoading.value = false
  }
}

const handleDowngrade = async () => {
  migrationLoading.value = true
  try {
    await executeMigrationDown(selectedTargetVersion.value)
    message.success(t('config.migrationSuccess'))
    await loadMigrationInfo()
  } catch (error: any) {
    message.error(t('config.migrationError', { error: error.message || error }))
  } finally {
    migrationLoading.value = false
  }
}

onMounted(() => {
  loadMigrationInfo()
})
</script>

<template>
  <div class="system-settings">
    <a-form
      layout="horizontal"
      :label-col="{ xs: { span: 24 }, sm: { span: 6 } }"
      :wrapper-col="{ xs: { span: 24 }, sm: { span: 18 } }"
      label-align="right"
    >
      <a-form-item :label="t('config.language')">
        <a-select :value="language" @update:value="emit('update:language', $event)">
          <a-select-option value="en">English</a-select-option>
          <a-select-option value="zh">中文</a-select-option>
        </a-select>
      </a-form-item>

      <a-form-item :label="t('config.theme')">
        <a-select :value="themeMode" @update:value="emit('update:themeMode', $event)">
          <a-select-option value="system">{{ t('config.themeSystem') }}</a-select-option>
          <a-select-option value="light">{{ t('config.themeLight') }}</a-select-option>
          <a-select-option value="dark">{{ t('config.themeDark') }}</a-select-option>
        </a-select>
      </a-form-item>

      <a-form-item :label="t('config.logLevel')">
        <a-select :value="logLevel" @update:value="emit('update:logLevel', $event)">
          <a-select-option value="trace">Trace</a-select-option>
          <a-select-option value="debug">Debug</a-select-option>
          <a-select-option value="info">Info</a-select-option>
          <a-select-option value="warn">Warn</a-select-option>
          <a-select-option value="error">Error</a-select-option>
        </a-select>
      </a-form-item>

      <a-form-item :label="t('config.enableAutoCheck')">
        <a-tooltip :title="!isCloudConfigured ? 'Only available when R2 or D1 is configured' : ''">
          <a-switch
            :checked="enableAutoCheck"
            :disabled="!isCloudConfigured"
            @update:checked="emit('update:enableAutoCheck', $event)"
          />
        </a-tooltip>
      </a-form-item>

      <a-form-item v-if="enableAutoCheck" :label="t('config.checkInterval')">
        <a-input-number
          :value="checkIntervalMins"
          :min="1"
          :max="1440"
          style="width: 100%"
          autocomplete="off"
          autocapitalize="none"
          autocorrect="off"
          spellcheck="false"
          @update:value="emit('update:checkIntervalMins', $event)"
        />
      </a-form-item>

      <!-- Dev Migration Section -->
      <template v-if="isDev">
        <a-divider />
        <h3 class="section-title">{{ t('config.dbMigration') }}</h3>

        <a-form-item :label="t('config.currentVersion')">
          <a-tag color="blue">{{ currentVersion || '0.0.0' }}</a-tag>
        </a-form-item>

        <a-form-item :label="t('config.targetVersion')">
          <a-select
            v-model:value="selectedTargetVersion"
            allow-clear
            :placeholder="t('config.latestVersion')"
          >
            <a-select-option v-for="v in availableVersions" :key="v" :value="v">
              {{ v }}
            </a-select-option>
          </a-select>
        </a-form-item>

        <a-form-item :wrapper-col="{ offset: 6, span: 18 }">
          <a-space>
            <a-button type="primary" :loading="migrationLoading" @click="handleUpgrade">
              {{ t('config.upgrade') }}
            </a-button>
            <a-button :loading="migrationLoading" @click="handleDowngrade">
              {{ t('config.downgrade') }}
            </a-button>
          </a-space>
        </a-form-item>
      </template>
    </a-form>
  </div>
</template>

<style scoped>
.section-title {
  margin-bottom: 16px;
  font-size: 16px;
  font-weight: 500;
}
</style>
