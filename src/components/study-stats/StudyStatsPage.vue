<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { message, theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import type { Book, StudyStatsFilters, StudyStatsPeriodType, StudyStatsResponse } from '../../types'
import { getBooks } from '../../lib/api'
import { getStudyStats } from '../../lib/api/studyTimer'

const { t } = useI18n()
const { useToken } = theme
const { token } = useToken()

const periodType = ref<StudyStatsPeriodType>('week')
const selectedBookId = ref<string>('all')
const selectedSeries = ref<string>('all')
const page = ref(1)
const pageSize = ref(10)
const loading = ref(false)
const stats = ref<StudyStatsResponse | null>(null)
const books = ref<Book[]>([])

const periodOptions: StudyStatsPeriodType[] = ['week', 'month', 'year']

const trendData = computed(() => stats.value?.trend || [])
const bookBreakdown = computed(() => stats.value?.bookBreakdown || [])
const seriesBreakdown = computed(() => stats.value?.seriesBreakdown || [])
const recentSessions = computed(() => stats.value?.recentSessions || [])

const trendMaxDuration = computed(() => {
  const durations = trendData.value.map((item) => item.duration)
  return durations.length ? Math.max(...durations) : 0
})

const totalDuration = computed(() => {
  return bookBreakdown.value.reduce((sum, item) => sum + item.duration, 0)
})

const totalRecent = computed(() => stats.value?.totalRecent || 0)
const totalPages = computed(() => Math.max(1, Math.ceil(totalRecent.value / pageSize.value)))

const filteredBookOptions = computed(() => {
  if (selectedSeries.value === 'all') return books.value
  const group = Number(selectedSeries.value)
  return books.value.filter((book) => book.book_group === group)
})

function formatSeconds(duration: number): string {
  const safe = Math.max(0, Math.floor(duration))
  const h = Math.floor(safe / 3600)
  const m = Math.floor((safe % 3600) / 60)
  const s = safe % 60
  return [h, m, s].map((part) => String(part).padStart(2, '0')).join(':')
}

function trendHeight(duration: number): string {
  if (trendMaxDuration.value <= 0) return '0%'
  const percent = (duration / trendMaxDuration.value) * 100
  return `${Math.max(4, Math.min(100, percent))}%`
}

function resolveSeriesLabel(seriesKey: string): string {
  if (seriesKey === 'vocabulary') return t('studyStats.seriesVocabulary')
  if (seriesKey === 'grammar') return t('studyStats.seriesGrammar')
  return t('studyStats.seriesOther')
}

function buildFilters(): StudyStatsFilters | undefined {
  const filters: StudyStatsFilters = {}

  if (selectedBookId.value !== 'all') {
    filters.bookId = Number(selectedBookId.value)
  }

  if (selectedSeries.value !== 'all') {
    filters.bookGroup = Number(selectedSeries.value)
  }

  if (!filters.bookId && !filters.bookGroup) {
    return undefined
  }

  return filters
}

async function refreshStats() {
  loading.value = true
  try {
    stats.value = await getStudyStats(periodType.value, buildFilters(), page.value, pageSize.value)
  } catch (error) {
    const errorText = error instanceof Error ? error.message : String(error)
    message.error(t('studyStats.loadFailed', { error: errorText }))
  } finally {
    loading.value = false
  }
}

function goPrevPage() {
  if (page.value <= 1 || loading.value) return
  page.value -= 1
}

function goNextPage() {
  if (page.value >= totalPages.value || loading.value) return
  page.value += 1
}

watch(selectedSeries, () => {
  if (selectedBookId.value === 'all') return
  const existing = filteredBookOptions.value.some(
    (book) => String(book.id) === selectedBookId.value,
  )
  if (!existing) {
    selectedBookId.value = 'all'
  }
})

watch([periodType, selectedBookId, selectedSeries], () => {
  page.value = 1
  void refreshStats()
})

watch(page, () => {
  void refreshStats()
})

onMounted(async () => {
  try {
    books.value = await getBooks()
  } catch {
    books.value = []
  }
  await refreshStats()
})
</script>

<template>
  <section class="study-stats-page h-full w-full">
    <div class="stats-shell">
      <div class="stats-toolbar">
        <div class="period-switch" role="tablist" :aria-label="t('studyStats.timeRange')">
          <button
            v-for="period in periodOptions"
            :key="period"
            type="button"
            class="period-btn"
            :class="{ active: periodType === period }"
            @click="periodType = period"
          >
            {{ t(`studyStats.period.${period}`) }}
          </button>
        </div>

        <div class="filters">
          <a-select v-model:value="selectedSeries" class="w-[160px]">
            <a-select-option value="all">{{ t('studyStats.allSeries') }}</a-select-option>
            <a-select-option value="1">{{ t('studyStats.seriesVocabulary') }}</a-select-option>
            <a-select-option value="2">{{ t('studyStats.seriesGrammar') }}</a-select-option>
          </a-select>

          <a-select v-model:value="selectedBookId" class="w-[220px]">
            <a-select-option value="all">{{ t('studyStats.allBooks') }}</a-select-option>
            <a-select-option
              v-for="book in filteredBookOptions"
              :key="book.id"
              :value="String(book.id)"
            >
              {{ book.title }}
            </a-select-option>
          </a-select>
        </div>
      </div>

      <div class="stats-grid">
        <article class="stats-card trend-card">
          <header class="card-title">{{ t('studyStats.trend') }}</header>
          <div v-if="loading" class="empty-state">{{ t('app.loading') }}</div>
          <div v-else-if="trendData.length === 0" class="empty-state">
            {{ t('studyStats.empty') }}
          </div>
          <div v-else class="trend-bars">
            <div v-for="item in trendData" :key="item.date" class="trend-bar-item">
              <div class="trend-bar-track">
                <div class="trend-bar-fill" :style="{ height: trendHeight(item.duration) }"></div>
              </div>
              <div class="trend-bar-label">{{ item.date.slice(5) }}</div>
              <div class="trend-bar-value">{{ formatSeconds(item.duration) }}</div>
            </div>
          </div>
        </article>

        <article class="stats-card">
          <header class="card-title">{{ t('studyStats.bookBreakdown') }}</header>
          <div v-if="loading" class="empty-state">{{ t('app.loading') }}</div>
          <div v-else-if="bookBreakdown.length === 0" class="empty-state">
            {{ t('studyStats.empty') }}
          </div>
          <ul v-else class="breakdown-list">
            <li v-for="item in bookBreakdown" :key="item.bookId" class="breakdown-item">
              <span class="breakdown-name">{{ item.bookTitle }}</span>
              <span class="breakdown-percent">
                {{
                  totalDuration > 0 ? ((item.duration / totalDuration) * 100).toFixed(1) : '0.0'
                }}%
              </span>
              <span class="breakdown-value">{{ formatSeconds(item.duration) }}</span>
            </li>
          </ul>
        </article>

        <article class="stats-card">
          <header class="card-title">{{ t('studyStats.seriesBreakdown') }}</header>
          <div v-if="loading" class="empty-state">{{ t('app.loading') }}</div>
          <div v-else-if="seriesBreakdown.length === 0" class="empty-state">
            {{ t('studyStats.empty') }}
          </div>
          <ul v-else class="breakdown-list">
            <li v-for="item in seriesBreakdown" :key="item.seriesKey" class="breakdown-item">
              <span class="breakdown-name">{{ resolveSeriesLabel(item.seriesKey) }}</span>
              <span class="breakdown-value">{{ formatSeconds(item.duration) }}</span>
            </li>
          </ul>
        </article>
      </div>

      <article class="stats-card recent-card">
        <header class="card-title">{{ t('studyStats.recentSessions') }}</header>

        <div v-if="loading" class="empty-state">{{ t('app.loading') }}</div>
        <div v-else-if="recentSessions.length === 0" class="empty-state">
          {{ t('studyStats.empty') }}
        </div>
        <template v-else>
          <div class="recent-table-wrap">
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
                <tr v-for="session in recentSessions" :key="session.id">
                  <td>{{ session.bookTitle }}</td>
                  <td>{{ session.unitName }}</td>
                  <td>{{ formatSeconds(session.duration) }}</td>
                  <td>{{ session.startAt.slice(0, 16).replace('T', ' ') }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="recent-pagination">
            <a-button size="small" :disabled="page <= 1 || loading" @click="goPrevPage">
              {{ t('reader.prevPage') }}
            </a-button>
            <span class="page-indicator">
              {{ t('studyStats.page') }} {{ page }} / {{ totalPages }}
            </span>
            <a-button size="small" :disabled="page >= totalPages || loading" @click="goNextPage">
              {{ t('reader.nextPage') }}
            </a-button>
          </div>
        </template>
      </article>
    </div>
  </section>
</template>

<style scoped>
.study-stats-page {
  padding: 16px;
  overflow-y: auto;
}

.stats-shell {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.stats-toolbar {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 12px;
  padding: 10px;
  border: 1px solid color-mix(in srgb, #ffffff 20%, transparent);
  border-radius: 16px;
  background: color-mix(in srgb, v-bind('token.colorBgContainer') 80%, transparent);
}

.period-switch {
  display: inline-flex;
  gap: 6px;
  padding: 4px;
  border-radius: 999px;
  background: v-bind('token.colorFillSecondary');
}

.period-btn {
  border: 0;
  padding: 6px 12px;
  border-radius: 999px;
  background: transparent;
  color: v-bind('token.colorTextSecondary');
  font-weight: 600;
  cursor: pointer;
}

.period-btn.active {
  background: v-bind('token.colorPrimary');
  color: #fff;
}

.filters {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.stats-grid {
  display: grid;
  grid-template-columns: 1.6fr 1fr 1fr;
  gap: 12px;
}

.stats-card {
  border: 1px solid color-mix(in srgb, #ffffff 15%, transparent);
  border-radius: 16px;
  padding: 12px;
  background: color-mix(in srgb, v-bind('token.colorBgContainer') 92%, transparent);
}

.card-title {
  font-size: 14px;
  font-weight: 700;
  margin-bottom: 10px;
}

.empty-state {
  min-height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: v-bind('token.colorTextSecondary');
}

.trend-bars {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  min-height: 200px;
}

.trend-bar-item {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.trend-bar-track {
  width: 100%;
  height: 132px;
  border-radius: 8px;
  background: v-bind('token.colorFillSecondary');
  display: flex;
  align-items: flex-end;
  overflow: hidden;
}

.trend-bar-fill {
  width: 100%;
  border-radius: 8px;
  background: linear-gradient(180deg, v-bind('token.colorPrimary'), v-bind('token.colorInfo'));
}

.trend-bar-label {
  font-size: 11px;
  color: v-bind('token.colorTextSecondary');
}

.trend-bar-value {
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  color: v-bind('token.colorText');
}

.breakdown-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.breakdown-item {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 8px;
  font-size: 13px;
  align-items: center;
}

.breakdown-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.breakdown-percent {
  color: v-bind('token.colorTextSecondary');
  font-variant-numeric: tabular-nums;
}

.breakdown-value {
  font-variant-numeric: tabular-nums;
}

.recent-table-wrap {
  overflow-x: auto;
}

.recent-table {
  width: 100%;
  border-collapse: collapse;
}

.recent-table th,
.recent-table td {
  padding: 8px 10px;
  border-bottom: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 65%, transparent);
  text-align: left;
  font-size: 13px;
}

.recent-table thead th {
  font-size: 12px;
  color: v-bind('token.colorTextSecondary');
  font-weight: 600;
}

.recent-pagination {
  margin-top: 10px;
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 10px;
}

.page-indicator {
  font-size: 12px;
  color: v-bind('token.colorTextSecondary');
}

@media (max-width: 1100px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }
}
</style>
