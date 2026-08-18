<script setup lang="ts">
import { computed } from 'vue'
import { theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import { getBookDisplayTitle } from '@/lib/book'
import type { Book, StudyStatsPeriodType } from '@/types'

const props = defineProps<{
  periodType: StudyStatsPeriodType
  selectedSeries: string
  selectedBookId: string
  books: Book[]
  durationText: string
  totalRecent: number
}>()

const emit = defineEmits<{
  (event: 'update:periodType', value: StudyStatsPeriodType): void
  (event: 'update:selectedSeries', value: string): void
  (event: 'update:selectedBookId', value: string): void
}>()

const { t } = useI18n()
const { token } = theme.useToken()
const periodOptions: StudyStatsPeriodType[] = ['week', 'month', 'year']
const filteredBooks = computed(() => {
  if (props.selectedSeries === 'all') return props.books
  const group = Number(props.selectedSeries)
  return props.books.filter((book) => book.book_group === group)
})

/** 重置统计筛选条件。 */
function resetFilters() {
  emit('update:selectedSeries', 'all')
  emit('update:selectedBookId', 'all')
}
</script>

<template>
  <div class="stats-toolbar">
    <div class="toolbar-main">
      <div class="period-switch" role="tablist" :aria-label="t('studyStats.timeRange')">
        <button
          v-for="period in periodOptions"
          :key="period"
          type="button"
          class="period-btn"
          :class="{ active: periodType === period }"
          @click="emit('update:periodType', period)"
        >
          {{ t(`studyStats.period.${period}`) }}
        </button>
      </div>

      <div class="summary-chips">
        <span class="summary-chip">{{ t('studyStats.duration') }} {{ durationText }}</span>
        <span class="summary-chip">{{ t('studyStats.recentSessions') }} {{ totalRecent }}</span>
      </div>
    </div>

    <div class="filters">
      <a-select
        :value="selectedSeries"
        class="series-select"
        @update:value="emit('update:selectedSeries', $event)"
      >
        <a-select-option value="all">{{ t('studyStats.allSeries') }}</a-select-option>
        <a-select-option value="1">{{ t('studyStats.seriesVocabulary') }}</a-select-option>
        <a-select-option value="2">{{ t('studyStats.seriesGrammar') }}</a-select-option>
      </a-select>

      <a-select
        :value="selectedBookId"
        class="book-select"
        @update:value="emit('update:selectedBookId', $event)"
      >
        <a-select-option value="all">{{ t('studyStats.allBooks') }}</a-select-option>
        <a-select-option v-for="book in filteredBooks" :key="book.id" :value="String(book.id)">
          {{ getBookDisplayTitle(book) }}
        </a-select-option>
      </a-select>

      <a-button class="reset-btn" @click="resetFilters">
        {{ t('studyStats.resetFilters') }}
      </a-button>
    </div>
  </div>
</template>

<style scoped>
.stats-toolbar {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  width: 100%;
  min-width: 0;
  gap: 12px;
  padding: 12px;
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 50%, transparent);
  border-radius: 12px;
  background: v-bind('token.colorBgContainer');
}

.toolbar-main,
.summary-chips,
.filters {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
}

.toolbar-main {
  gap: 10px;
}

.period-switch {
  display: inline-flex;
  gap: 6px;
  padding: 4px;
  border-radius: 999px;
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 40%, transparent);
}

.period-btn {
  border: 0;
  padding: 8px 14px;
  border-radius: 999px;
  background: transparent;
  color: v-bind('token.colorTextSecondary');
  font-weight: 600;
  cursor: pointer;
  transition:
    color 0.2s ease,
    background-color 0.2s ease,
    box-shadow 0.2s ease;
}

.period-btn.active {
  background: v-bind('token.colorPrimary');
  color: #fff;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.summary-chips {
  gap: 6px;
}

.summary-chip {
  height: 30px;
  border-radius: 999px;
  padding: 0 12px;
  display: inline-flex;
  align-items: center;
  font-size: 12px;
  font-weight: 700;
  color: v-bind('token.colorPrimary');
  background: color-mix(in srgb, v-bind('token.colorPrimaryBg') 40%, transparent);
  border: 1px solid color-mix(in srgb, v-bind('token.colorPrimaryBorder') 30%, transparent);
}

.filters {
  align-items: flex-start;
  flex: 1 1 auto;
  min-width: 0;
  gap: 8px;
  justify-content: flex-end;
}

/* 保留组件库原生下箭头，并沿用组件库默认的垂直居中方式。 */
.filters :deep(.ant-select-arrow) {
  color: inherit;
  font-size: inherit;
}

.series-select {
  width: 160px;
}

.book-select {
  width: 220px;
}

.reset-btn {
  min-height: 32px;
}

@media (max-width: 960px) {
  .stats-toolbar {
    flex-direction: column-reverse;
    align-items: stretch;
    padding: 8px 10px;
    gap: 8px;
  }

  .toolbar-main {
    width: 100%;
    justify-content: space-between;
  }

  .filters {
    width: 100%;
    justify-content: flex-start;
    gap: 6px;
  }

  .series-select,
  .book-select {
    flex: 1 1 140px;
    min-width: 120px;
    width: auto;
  }

  .reset-btn {
    flex: 0 0 auto;
  }

  .summary-chip {
    font-size: 11px;
  }
}

@media (max-width: 480px) {
  .filters {
    flex-direction: column;
  }

  .series-select,
  .book-select {
    flex: 0 0 auto;
  }

  .series-select,
  .book-select,
  .reset-btn {
    width: 100%;
  }
}
</style>
