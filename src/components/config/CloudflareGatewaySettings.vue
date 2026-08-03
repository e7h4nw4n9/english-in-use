<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface Props {
  gatewayConfig: {
    base_url: string
    access_token: string
  }
}

defineProps<Props>()
</script>

<template>
  <a-form-item :label="t('config.gatewayUrl' as any)">
    <a-input
      v-model:value="gatewayConfig.base_url"
      placeholder="https://gateway.example.com"
      autocomplete="off"
      autocapitalize="none"
      autocorrect="off"
      spellcheck="false"
    />
  </a-form-item>
  <a-form-item :label="t('config.gatewayToken' as any)">
    <a-input-password
      v-model:value="gatewayConfig.access_token"
      autocomplete="off"
      autocapitalize="none"
      autocorrect="off"
      spellcheck="false"
    />
  </a-form-item>
</template>
