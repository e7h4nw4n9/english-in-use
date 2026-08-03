<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { FolderOpenOutlined } from '@ant-design/icons-vue'
import type { BookSourceType } from '../../types'
import CloudflareGatewaySettings from './CloudflareGatewaySettings.vue'

const { t } = useI18n()

interface Props {
  sourceType: BookSourceType
  localBookPath: string
  gatewayConfig: {
    base_url: string
    access_token: string
  }
  isTesting: boolean
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:sourceType', value: BookSourceType): void
  (e: 'update:localBookPath', value: string): void
  (e: 'select-folder'): void
  (e: 'test-connection'): void
}>()
</script>

<template>
  <div>
    <a-form
      layout="horizontal"
      :label-col="{ xs: { span: 24 }, sm: { span: 6 } }"
      :wrapper-col="{ xs: { span: 24 }, sm: { span: 18 } }"
      label-align="right"
    >
      <a-form-item :label="t('config.sourceType')">
        <a-radio-group
          :value="sourceType"
          button-style="solid"
          @update:value="emit('update:sourceType', $event)"
        >
          <a-radio-button value="Local">{{ t('config.localFolder') }}</a-radio-button>
          <a-radio-button value="CloudflareGateway">{{
            t('config.cloudGateway' as any)
          }}</a-radio-button>
        </a-radio-group>
      </a-form-item>

      <div v-if="sourceType === 'Local'">
        <a-form-item :label="t('config.folderPath')">
          <a-input
            :value="localBookPath"
            :placeholder="t('config.folderPath')"
            autocomplete="off"
            autocapitalize="none"
            autocorrect="off"
            spellcheck="false"
            @update:value="emit('update:localBookPath', $event)"
          >
            <template #addonAfter>
              <FolderOpenOutlined @click="emit('select-folder')" class="cursor-pointer" />
            </template>
          </a-input>
        </a-form-item>
      </div>

      <div v-else>
        <CloudflareGatewaySettings :gateway-config="gatewayConfig" />
      </div>
    </a-form>

    <div v-if="sourceType === 'CloudflareGateway'" class="form-footer-actions">
      <a-button @click="emit('test-connection')" :loading="isTesting">
        {{ t('config.testConnection') }}
      </a-button>
    </div>
  </div>
</template>

<style scoped>
.form-footer-actions {
  margin-top: 24px;
  padding-left: 25%;
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
  color: #1677ff; /* Primary color fallback */
}
</style>
