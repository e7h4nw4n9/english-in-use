<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatIsoToLocalMinute } from '@/lib/datetime'
import type { StudySessionListItem, UpdateStudySessionPayload } from '@/types'

const props = defineProps<{
  open: boolean
  session: StudySessionListItem | null
  loading: boolean
}>()

const emit = defineEmits<{
  (event: 'update:open', value: boolean): void
  (event: 'confirm', payload: UpdateStudySessionPayload): void
}>()

const { t } = useI18n()
const hours = ref(0)
const minutes = ref(0)
const seconds = ref(0)
const assignedResourceId = ref('')

const unitOptions = computed(() => {
  if (!props.session) return []
  const options = [...props.session.visitedUnits]
  if (!options.some((unit) => unit.resourceId === props.session?.resourceId)) {
    options.push({
      resourceId: props.session.resourceId,
      unitName: props.session.unitName,
    })
  }
  return options
})
const duration = computed(() => hours.value * 3600 + minutes.value * 60 + seconds.value)
const canSubmit = computed(() =>
  Boolean(props.session && assignedResourceId.value && duration.value > 0),
)
const endAtPreview = computed(() => {
  if (!props.session || duration.value <= 0) return ''
  const startAt = new Date(props.session.startAt).getTime()
  if (!Number.isFinite(startAt)) return ''
  return formatIsoToLocalMinute(new Date(startAt + duration.value * 1000).toISOString())
})

watch(
  () => [props.open, props.session] as const,
  ([open, session]) => {
    if (!open || !session) return
    hours.value = Math.floor(session.duration / 3600)
    minutes.value = Math.floor((session.duration % 3600) / 60)
    seconds.value = session.duration % 60
    assignedResourceId.value = session.resourceId
  },
  { immediate: true },
)

/** 提交学习会话修改。 */
function submit() {
  if (!props.session || !canSubmit.value) return
  emit('confirm', {
    sessionId: props.session.id,
    duration: duration.value,
    assignedResourceId: assignedResourceId.value,
  })
}
</script>

<template>
  <a-modal
    :open="open"
    :title="t('studyStats.editTitle')"
    :confirm-loading="loading"
    :ok-button-props="{ disabled: !canSubmit }"
    :ok-text="t('common.ok')"
    :cancel-text="t('common.cancel')"
    destroy-on-close
    @ok="submit"
    @update:open="emit('update:open', $event)"
  >
    <div v-if="session" class="edit-form">
      <label class="field-label" for="study-session-unit">{{ t('studyStats.unit') }}</label>
      <a-select id="study-session-unit" v-model:value="assignedResourceId">
        <a-select-option
          v-for="unit in unitOptions"
          :key="unit.resourceId"
          :value="unit.resourceId"
        >
          {{ unit.unitName }}
        </a-select-option>
      </a-select>

      <fieldset class="duration-fieldset">
        <legend class="field-label">{{ t('studyStats.duration') }}</legend>
        <div class="duration-inputs">
          <label>
            <span>{{ t('studyStats.hours') }}</span>
            <a-input-number v-model:value="hours" :min="0" :precision="0" />
          </label>
          <label>
            <span>{{ t('studyStats.minutes') }}</span>
            <a-input-number v-model:value="minutes" :min="0" :max="59" :precision="0" />
          </label>
          <label>
            <span>{{ t('studyStats.seconds') }}</span>
            <a-input-number v-model:value="seconds" :min="0" :max="59" :precision="0" />
          </label>
        </div>
      </fieldset>

      <div class="time-preview">
        <span>{{ t('studyStats.startTime') }}：{{ formatIsoToLocalMinute(session.startAt) }}</span>
        <span>{{ t('studyStats.newEndTime') }}：{{ endAtPreview }}</span>
      </div>
    </div>
  </a-modal>
</template>

<style scoped>
.edit-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.field-label {
  font-size: 13px;
  font-weight: 600;
}

.duration-fieldset {
  min-width: 0;
  margin: 0;
  padding: 0;
  border: 0;
}

.duration-inputs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin-top: 8px;
}

.duration-inputs label {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
}

.duration-inputs :deep(.ant-input-number) {
  width: 100%;
}

.time-preview {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(127, 127, 127, 0.08);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

@media (max-width: 480px) {
  .duration-inputs {
    grid-template-columns: 1fr;
  }
}
</style>
