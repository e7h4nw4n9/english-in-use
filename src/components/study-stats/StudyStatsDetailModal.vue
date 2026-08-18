<script setup lang="ts">
import { computed } from 'vue'
import { theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import { formatIsoToLocalMinute } from '@/lib/datetime'
import type { StudySessionListItem } from '@/types'

const props = defineProps<{
  open: boolean
  loading: boolean
  date: string
  sessions: StudySessionListItem[]
}>()

const emit = defineEmits<{
  (event: 'update:open', value: boolean): void
}>()

const { t } = useI18n()
const { token } = theme.useToken()
const totalDuration = computed(() =>
  props.sessions.reduce((sum, session) => sum + Math.max(0, session.duration), 0),
)

/** 将秒数格式化为时分秒。
 * @param duration - 学习时长，单位为秒。
 */
function formatFullSeconds(duration: number): string {
  const safe = Math.max(0, Math.floor(duration))
  const parts = [Math.floor(safe / 3600), Math.floor((safe % 3600) / 60), safe % 60]
  return parts.map((part) => String(part).padStart(2, '0')).join(':')
}
</script>

<template>
  <a-modal
    :open="open"
    :title="date + ' ' + t('studyStats.duration')"
    :footer="null"
    width="700px"
    @update:open="emit('update:open', $event)"
  >
    <a-spin :spinning="loading">
      <div class="modal-content">
        <div class="detail-duration-summary">
          <span>{{ t('studyStats.totalDuration') }}</span>
          <strong>{{ formatFullSeconds(totalDuration) }}</strong>
        </div>
        <div v-if="sessions.length === 0" class="empty-state">{{ t('studyStats.empty') }}</div>
        <div v-else class="recent-table-wrap">
          <table class="recent-table">
            <thead>
              <tr>
                <th>{{ t('studyStats.book') }}</th>
                <th>{{ t('studyStats.unit') }}</th>
                <th>{{ t('studyStats.duration') }}</th>
                <th>{{ t('studyStats.timeRange') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="session in sessions" :key="session.id">
                <td>{{ session.bookTitle }}</td>
                <td>{{ session.unitName }}</td>
                <td>{{ formatFullSeconds(session.duration) }}</td>
                <td>{{ formatIsoToLocalMinute(session.startAt).split(' ')[1] }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </a-spin>
  </a-modal>
</template>

<style scoped>
.modal-content {
  min-height: 200px;
}

.detail-duration-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  color: v-bind('token.colorTextSecondary');
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 30%, transparent);
}

.detail-duration-summary strong {
  color: v-bind('token.colorText');
  font-variant-numeric: tabular-nums;
}

.empty-state {
  min-height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: v-bind('token.colorTextSecondary');
}

.recent-table-wrap {
  max-width: 100%;
  overflow-x: auto;
}

.recent-table {
  width: 100%;
  border-collapse: collapse;
}

.recent-table th,
.recent-table td {
  padding: 12px;
  border-bottom: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 40%, transparent);
  text-align: left;
  font-size: 13px;
}

.recent-table thead th {
  font-size: 12px;
  color: v-bind('token.colorTextSecondary');
  font-weight: 700;
}
</style>
