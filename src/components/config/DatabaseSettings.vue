<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { CopyOutlined, FolderOpenOutlined, ReloadOutlined } from '@ant-design/icons-vue'
import type { DatabaseType } from '../../types'

const { t } = useI18n()

interface Props {
  dbType: DatabaseType
  sqlitePath: string
  d1Config: {
    account_id: string
    database_id: string
    api_token: string
  }
  isTesting: boolean
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:dbType', value: DatabaseType): void
  (e: 'update:sqlitePath', value: string): void
  (e: 'choose-sqlite-path'): void
  (e: 'restore-default-sqlite-path'): void
  (e: 'copy-path', value: string): void
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
      <a-form-item :label="t('config.databaseType')">
        <a-radio-group
          :value="dbType"
          button-style="solid"
          @update:value="emit('update:dbType', $event)"
        >
          <a-radio-button value="SQLite">{{ t('config.localSqlite') }}</a-radio-button>
          <a-radio-button value="CloudflareD1">{{ t('config.cloudD1') }}</a-radio-button>
        </a-radio-group>
      </a-form-item>

      <div v-if="dbType === 'SQLite'">
        <a-form-item :label="t('config.filePath')">
          <a-tooltip :title="sqlitePath" placement="topLeft">
            <a-input
              :value="sqlitePath"
              :placeholder="t('config.filePath')"
              autocomplete="off"
              autocapitalize="none"
              autocorrect="off"
              spellcheck="false"
              @update:value="emit('update:sqlitePath', $event)"
            >
              <template #addonAfter>
                <span class="sqlite-action-icons">
                  <a-tooltip :title="t('config.browse')">
                    <FolderOpenOutlined
                      @click="emit('choose-sqlite-path')"
                      class="choose-path-icon cursor-pointer"
                    />
                  </a-tooltip>
                  <a-tooltip
                    :title="t('config.restoreDefaultPath' as any) || 'Restore Default Path'"
                  >
                    <ReloadOutlined
                      @click="emit('restore-default-sqlite-path')"
                      class="restore-default-icon cursor-pointer"
                    />
                  </a-tooltip>
                  <a-tooltip :title="t('common.copy' as any) || 'Copy'">
                    <CopyOutlined
                      @click="emit('copy-path', sqlitePath)"
                      class="copy-path-icon cursor-pointer"
                    />
                  </a-tooltip>
                </span>
              </template>
            </a-input>
          </a-tooltip>
        </a-form-item>
      </div>

      <div v-else-if="dbType === 'CloudflareD1'">
        <a-form-item :label="t('config.accountId')">
          <a-input
            v-model:value="d1Config.account_id"
            autocomplete="off"
            autocapitalize="none"
            autocorrect="off"
            spellcheck="false"
          />
        </a-form-item>
        <a-form-item :label="t('config.databaseId')">
          <a-input
            v-model:value="d1Config.database_id"
            autocomplete="off"
            autocapitalize="none"
            autocorrect="off"
            spellcheck="false"
          />
        </a-form-item>
        <a-form-item :label="t('config.apiToken')">
          <a-input-password
            v-model:value="d1Config.api_token"
            autocomplete="off"
            autocapitalize="none"
            autocorrect="off"
            spellcheck="false"
          />
        </a-form-item>
      </div>
    </a-form>

    <div class="form-footer-actions">
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
  color: #1677ff;
}

.sqlite-action-icons {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
</style>
