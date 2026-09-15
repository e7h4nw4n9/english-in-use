<script setup lang="ts">
import { theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import type { StudyArrangementReview } from '@/lib/api/studyArrangements'
import { formatIsoToLocalMinute, formatLocalDate } from '@/lib/datetime'

defineProps<{ reviews?: StudyArrangementReview[]; loading?: boolean; error?: boolean }>()
defineEmits<{ retry: [] }>()
const { t } = useI18n()
const { token } = theme.useToken()
/** 根据任务完成状态和本地日期显示逾期提示；review 为复习记录。 */
function reviewStatus(review: StudyArrangementReview) {
  return review.taskStatus === 1
    ? 'done'
    : review.scheduledDate < formatLocalDate()
      ? 'overdue'
      : 'pending'
}
</script>
<template>
  <div class="review-details" @click.stop>
    <p v-if="loading" role="status">{{ t('studyArrangements.loading') }}</p>
    <a-alert v-else-if="error" type="error" :message="t('studyArrangements.loadError')">
      <template #action
        ><a-button @click="$emit('retry')">{{ t('studyArrangements.retry') }}</a-button></template
      >
    </a-alert>
    <p v-else-if="!reviews?.length">{{ t('studyArrangements.noReviews') }}</p>
    <ul v-else class="review-list" :aria-label="t('studyArrangements.reviews')">
      <li v-for="review in reviews" :key="review.taskId">
        <strong>{{ t('studyArrangements.stage', { n: review.reviewStage }) }}</strong>
        <span>{{ t('studyArrangements.date') }}：{{ review.scheduledDate }}</span>
        <a-tag
          :color="
            reviewStatus(review) === 'done'
              ? 'green'
              : reviewStatus(review) === 'overdue'
                ? 'red'
                : 'default'
          "
          >{{ t(`studyArrangements.${reviewStatus(review)}`) }}</a-tag
        >
        <span class="review-completed"
          >{{ t('studyArrangements.completedAt') }}：{{
            review.completedAt ? formatIsoToLocalMinute(review.completedAt) : '—'
          }}</span
        >
      </li>
    </ul>
  </div>
</template>
<style scoped>
.review-list {
  padding: 0;
  margin: 0;
  list-style: none;
}
.review-list li {
  display: grid;
  grid-template-columns: 90px minmax(160px, 1fr) 90px minmax(210px, 1fr);
  align-items: center;
  gap: 10px;
  padding: 12px 0;
  border-bottom: 1px solid v-bind('token.colorBorderSecondary');
}
.review-list li:last-child {
  border-bottom: 0;
}
.review-list .ant-tag {
  justify-self: start;
  margin: 0;
}
.review-completed {
  color: v-bind('token.colorTextSecondary');
}
@media (max-width: 767px) {
  .review-list li {
    grid-template-columns: 1fr auto;
    gap: 8px;
  }
  .review-list li > span:nth-child(2),
  .review-completed {
    grid-column: 1 / -1;
  }
  .review-list .ant-tag {
    grid-column: 2;
    grid-row: 1;
  }
  .review-details {
    min-width: 0;
    font-size: 12px;
  }
}
</style>
