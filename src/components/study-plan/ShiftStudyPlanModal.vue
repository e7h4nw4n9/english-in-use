<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { theme } from 'ant-design-vue'
import type { StudyTaskItem } from '@/types'
import { formatUnitTitle } from '@/lib/unitTitle'
import { addDays, formatDate, parseDate } from '@/composables/study-plan/studyPlanCalendar'

const props = defineProps<{
  open: boolean
  task: StudyTaskItem | null
  loading: boolean
  today: string
}>()

const emit = defineEmits<{
  (event: 'update:open', value: boolean): void
  (event: 'confirm', task: StudyTaskItem, offsetDays: number): void
}>()

const { t } = useI18n()
const { token } = theme.useToken()
const direction = ref<'advance' | 'delay'>('delay')
const days = ref(1)

const offsetDays = computed(() => (direction.value === 'advance' ? -days.value : days.value))
const targetDate = computed(() => {
  if (!props.task || !Number.isInteger(days.value) || days.value < 1) return ''
  return formatDate(addDays(parseDate(props.task.scheduledDate), offsetDays.value))
})
const invalidTarget = computed(() => Boolean(targetDate.value && targetDate.value < props.today))
const canSubmit = computed(() =>
  Boolean(props.task && Number.isInteger(days.value) && days.value >= 1 && !invalidTarget.value),
)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    direction.value = 'delay'
    days.value = 1
  },
)

/** 提交计划平移请求。 */
function submit() {
  if (!props.task || !canSubmit.value) return
  emit('confirm', props.task, offsetDays.value)
}
</script>

<template>
  <a-modal
    :open="open"
    :title="t('studyPlan.shiftTitle')"
    :confirm-loading="loading"
    :ok-button-props="{ disabled: !canSubmit }"
    :ok-text="t('studyPlan.shiftConfirm')"
    :cancel-text="t('common.cancel')"
    destroy-on-close
    @ok="submit"
    @update:open="emit('update:open', $event)"
  >
    <div v-if="task" class="shift-form">
      <div class="shift-unit">{{ formatUnitTitle(task.resourceId, task.unitName) }}</div>

      <label class="field-label">{{ t('studyPlan.shiftDirection') }}</label>
      <a-radio-group v-model:value="direction" button-style="solid">
        <a-radio-button value="advance">{{ t('studyPlan.advance') }}</a-radio-button>
        <a-radio-button value="delay">{{ t('studyPlan.delay') }}</a-radio-button>
      </a-radio-group>

      <label class="field-label" for="study-plan-shift-days">
        {{ t('studyPlan.shiftDays') }}
      </label>
      <a-input-number
        id="study-plan-shift-days"
        v-model:value="days"
        :min="1"
        :precision="0"
        class="days-input"
      />

      <div class="date-preview">
        <span>{{ task.scheduledDate }}</span>
        <span aria-hidden="true">→</span>
        <strong>{{ targetDate }}</strong>
      </div>
      <a-alert
        v-if="invalidTarget"
        type="error"
        show-icon
        :message="t('studyPlan.shiftBeforeToday')"
      />
    </div>
  </a-modal>
</template>

<style scoped>
.shift-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.shift-unit {
  font-weight: 700;
}

.field-label {
  color: v-bind('token.colorTextSecondary');
  font-size: 13px;
  font-weight: 600;
}

.days-input {
  width: 100%;
}

.date-preview {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 10px;
  border-radius: 8px;
  background: rgba(127, 127, 127, 0.08);
  font-variant-numeric: tabular-nums;
}
</style>
