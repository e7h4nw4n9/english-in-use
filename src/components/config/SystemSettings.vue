<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { SystemConfig } from '../../types'

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
</script>

<template>
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
  </a-form>
</template>
