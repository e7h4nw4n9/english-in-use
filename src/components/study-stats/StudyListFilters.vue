<script setup lang="ts">
import { computed } from 'vue'
import { DatePicker } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import type { Book } from '@/types'
import { getBookDisplayTitle } from '@/lib/book'
import type { StudyArrangementQuery } from '@/lib/api/studyArrangements'

const ARangePicker = DatePicker.RangePicker
const props = defineProps<{
  books: Book[]
  showDates: boolean
  showStatus: boolean
}>()
const series = defineModel<string>('series', { required: true })
const bookId = defineModel<string>('bookId', { required: true })
const dateRange = defineModel<[string, string]>('dateRange', { required: true })
const status = defineModel<StudyArrangementQuery['status']>('status', { required: true })
const emit = defineEmits<{ reset: [] }>()
const { t } = useI18n()
const bookOptions = computed(() => [
  { value: 'all', label: t('studyStats.allBooks') },
  ...props.books
    .filter((book) => series.value === 'all' || book.book_group === Number(series.value))
    .map((book) => ({ value: String(book.id), label: getBookDisplayTitle(book) })),
])
const statusOptions = computed(() =>
  ['all', 'scheduled', 'active', 'ended'].map((value) => ({
    value,
    label: t(`studyArrangements.${value}`),
  })),
)
</script>

<template>
  <div class="list-filters">
    <a-select v-model:value="series" :aria-label="t('studyStats.allSeries')" class="series-filter">
      <a-select-option value="all">{{ t('studyStats.allSeries') }}</a-select-option>
      <a-select-option value="1">{{ t('studyStats.seriesVocabulary') }}</a-select-option>
      <a-select-option value="2">{{ t('studyStats.seriesGrammar') }}</a-select-option>
    </a-select>
    <a-select
      v-model:value="bookId"
      :options="bookOptions"
      :aria-label="t('studyStats.book')"
      class="book-filter"
    />
    <a-range-picker
      v-if="showDates"
      v-model:value="dateRange"
      value-format="YYYY-MM-DD"
      :allow-clear="false"
      :aria-label="t('studyStats.dateRange')"
      class="date-filter"
    />
    <a-select
      v-if="showStatus"
      v-model:value="status"
      :options="statusOptions"
      :aria-label="t('studyArrangements.status')"
      class="status-filter"
    />
    <a-button @click="emit('reset')">{{ t('studyStats.resetFilters') }}</a-button>
  </div>
</template>

<style scoped>
.list-filters {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 16px;
}
.series-filter {
  width: 160px;
}
.book-filter {
  width: 240px;
}
.date-filter {
  width: 270px;
}
.status-filter {
  width: 150px;
}
@media (max-width: 600px) {
  .series-filter,
  .book-filter,
  .date-filter,
  .status-filter {
    width: 100%;
  }
}
</style>
