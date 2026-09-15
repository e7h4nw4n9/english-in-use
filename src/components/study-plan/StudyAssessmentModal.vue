<script setup lang="ts">
import { computed, ref, watch, onBeforeUnmount } from 'vue'
import { theme } from 'ant-design-vue'
import type { StudyTaskItem, StudyAssessmentPreview, StudyMasteryRating } from '../../types'
import { formatUnitTitle } from '@/lib/unitTitle'
import { getStudyAssessmentPreview } from '@/lib/api/studyPlan'
import { formatLocalDate } from '@/lib/datetime'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ task: StudyTaskItem | null; loading: boolean }>()
const { token } = theme.useToken()
const { t } = useI18n()
const emit = defineEmits<{
  cancel: []
  submit: [rating: StudyMasteryRating, finishPlan: boolean | undefined, expectedRevision: string]
}>()
const rating = ref<'forgotten' | 'hard' | 'good' | 'mastered' | null>(null)
const finishPlan = ref<boolean | undefined>()
const preview = ref<StudyAssessmentPreview | null>(null)
const previewLoading = ref(false)
const previewError = ref(false)
const refreshedDate = ref(false)
let previewRequest = 0
const selectedOption = computed(() =>
  preview.value?.options.find((option) => option.rating === rating.value),
)
const needsDecision = computed(() => selectedOption.value?.requiresFinishDecision ?? false)
const nextDays = computed(() => selectedOption.value?.intervalDays)
const canSubmit = computed(
  () =>
    !props.loading &&
    !previewLoading.value &&
    Boolean(selectedOption.value) &&
    (!needsDecision.value || finishPlan.value !== undefined),
)

/** 重新获取后端预览，忽略已经关闭或切换的弹窗请求。 */
async function loadPreview() {
  const request = ++previewRequest
  const task = props.task
  preview.value = null
  previewError.value = false
  previewLoading.value = Boolean(task)
  if (!task) return
  try {
    const result = await getStudyAssessmentPreview(task.taskId)
    if (request === previewRequest) preview.value = result
  } catch {
    if (request === previewRequest) previewError.value = true
  } finally {
    if (request === previewRequest) previewLoading.value = false
  }
}
watch(
  () => props.task,
  () => {
    rating.value = null
    finishPlan.value = undefined
    refreshedDate.value = false
    void loadPreview()
  },
  { immediate: true },
)
onBeforeUnmount(() => {
  previewRequest++
})
watch(rating, () => {
  finishPlan.value = undefined
})

/** 仅在评价与必要的结束选择齐全后提交。 */
async function submit() {
  if (!canSubmit.value || !rating.value || !preview.value) return
  if (preview.value.localDate !== formatLocalDate()) {
    rating.value = null
    finishPlan.value = undefined
    refreshedDate.value = true
    await loadPreview()
    return
  }
  emit(
    'submit',
    rating.value,
    needsDecision.value ? finishPlan.value : undefined,
    preview.value.revision,
  )
}
</script>

<template>
  <a-modal
    :open="Boolean(task)"
    ok-text="完成并保存安排"
    cancel-text="取消"
    :confirm-loading="loading"
    :closable="!loading"
    :mask-closable="false"
    :keyboard="!loading"
    :cancel-button-props="{ disabled: loading }"
    :ok-button-props="{ disabled: !canSubmit }"
    @cancel="emit('cancel')"
    @ok="submit"
  >
    <template #title>
      <div class="assessment-header" :style="{ color: token.colorText }">
        <div>掌握情况评估</div>
        <p v-if="task" class="assessment-unit">
          {{ formatUnitTitle(task.resourceId, task.unitName) }}
        </p>
      </div>
    </template>
    <!-- 弹窗内容通过 Teleport 渲染，主题变量直接绑定到内部容器。 -->
    <div
      class="assessment-body"
      :style="{
        '--assessment-text': token.colorText,
        '--assessment-secondary': token.colorTextSecondary,
        '--assessment-fill': token.colorFillAlter,
      }"
    >
      <p v-if="previewLoading">{{ t('studyPlanRules.previewLoading') }}</p>
      <a-alert v-if="previewError" type="error" :message="t('studyPlanRules.previewError')">
        <template #action
          ><a-button class="assessment-retry" size="small" @click="loadPreview">{{
            t('studyPlanRules.retry')
          }}</a-button></template
        >
      </a-alert>
      <p v-if="refreshedDate" class="assessment-secondary">{{ t('studyPlanRules.dateChanged') }}</p>
      <a-radio-group
        class="assessment-ratings"
        v-model:value="rating"
        :disabled="loading"
        aria-label="掌握情况"
      >
        <a-radio-button value="forgotten">未掌握</a-radio-button>
        <a-radio-button value="hard">不熟练</a-radio-button>
        <a-radio-button value="good">熟练</a-radio-button>
        <a-radio-button value="mastered">已掌握</a-radio-button>
      </a-radio-group>
      <div v-if="needsDecision" class="assessment-decision">
        <p>已连续完成 3 次巩固复习，是否结束该学习计划？</p>
        <a-radio-group v-model:value="finishPlan" :disabled="loading" aria-label="是否结束学习计划">
          <a-radio :value="true">结束计划</a-radio>
          <a-radio :value="false">继续学习</a-radio>
        </a-radio-group>
      </div>
      <div
        v-if="selectedOption && !(needsDecision && finishPlan === true)"
        class="assessment-hints"
        aria-live="polite"
      >
        <p>
          下次复习：本次完成日期的 <strong>{{ nextDays }} 天后</strong>。
        </p>
        <p class="assessment-secondary">{{ selectedOption.scheduledDate }}</p>
        <p v-if="preview?.sameDay" class="assessment-secondary">
          {{ t('studyPlanRules.sameDay') }}
        </p>
        <p v-else-if="preview?.legacyDate" class="assessment-secondary">
          {{ t('studyPlanRules.legacy') }}
        </p>
        <p v-if="rating === 'mastered' && !needsDecision" class="assessment-secondary">
          连续完成三次巩固且均已掌握后，可选择结束计划。
        </p>
      </div>
    </div>
  </a-modal>
</template>

<style scoped>
.assessment-header {
  padding-right: 28px;
}

.assessment-unit {
  margin: 8px 0 0;
  font-weight: 700;
  line-height: 1.6;
  overflow-wrap: anywhere;
  color: inherit;
}

.assessment-body {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 16px 0 12px;
}

.assessment-ratings {
  display: flex;
}

.assessment-ratings :deep(.ant-radio-button-wrapper) {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  min-height: 44px;
  padding: 0 8px;
}

.assessment-hints {
  padding: 16px;
  border-radius: 8px;
  background: var(--assessment-fill);
  color: var(--assessment-text);
  line-height: 1.8;
}

.assessment-hints p,
.assessment-decision p {
  margin: 0;
}

.assessment-hints p + p {
  margin-top: 8px;
}

.assessment-secondary {
  color: var(--assessment-secondary);
}

.assessment-decision {
  display: flex;
  flex-direction: column;
  gap: 12px;
  line-height: 1.8;
}

@media (max-width: 360px) {
  .assessment-ratings {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .assessment-ratings :deep(.ant-radio-button-wrapper) {
    border-inline-start-width: 1px;
    border-radius: 6px;
  }

  .assessment-ratings :deep(.ant-radio-button-wrapper::before) {
    display: none;
  }
}
</style>
