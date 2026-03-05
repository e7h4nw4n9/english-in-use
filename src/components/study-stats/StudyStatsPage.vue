<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { message, theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import type {
  Book,
  StudySessionListItem,
  StudyStatsFilters,
  StudyStatsPeriodType,
  StudyStatsResponse,
  StudyStatsTrendItem,
} from '../../types'
import { getBooks } from '../../lib/api'
import { getStudySessionsByDate, getStudyStats } from '../../lib/api/studyTimer'
import { formatIsoToLocalMinute, generateDateRange, generateMonthRange } from '../../lib/datetime'

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

// 详情弹窗
const detailModalVisible = ref(false)
const detailLoading = ref(false)
const detailDate = ref('')
const detailSessions = ref<StudySessionListItem[]>([])

const periodOptions: StudyStatsPeriodType[] = ['week', 'month', 'year']

const trendData = computed<StudyStatsTrendItem[]>(() => {
  if (!stats.value) return []
  const { rangeStart, rangeEnd, trend } = stats.value
  if (!rangeStart || !rangeEnd) return trend

  if (periodType.value === 'year') {
    // 年视图：按月统计
    const months = generateMonthRange(rangeStart, rangeEnd)
    const trendMap = new Map<string, number>()

    // 聚合现有趋势数据到月份
    trend.forEach((item) => {
      const monthKey = item.date.slice(0, 7) // YYYY-MM
      const current = trendMap.get(monthKey) || 0
      trendMap.set(monthKey, current + item.duration)
    })

    return months.map((month) => ({
      date: month,
      duration: trendMap.get(month) || 0,
    }))
  }

  // 周/月视图：按日统计
  const range = generateDateRange(rangeStart, rangeEnd)
  const trendMap = new Map(trend.map((item) => [item.date, item.duration]))

  return range.map((date) => ({
    date,
    duration: trendMap.get(date) || 0,
  }))
})

const bookBreakdown = computed(() => stats.value?.bookBreakdown || [])
const seriesBreakdown = computed(() => stats.value?.seriesBreakdown || [])
const recentSessions = computed(() => stats.value?.recentSessions || [])

const trendMaxDuration = computed(() => {
  const durations = trendData.value.map((item) => item.duration)
  return durations.length ? Math.max(...durations) : 0
})

// Y轴刻度：根据最大时长分4段
const yAxisLabels = computed(() => {
  const max = trendMaxDuration.value
  if (max <= 0) return ['0s']
  const steps = 4
  const labels: string[] = []
  for (let i = steps; i >= 0; i--) {
    labels.push(formatSeconds((max * i) / steps))
  }
  return labels
})

const totalDuration = computed(() => {
  return bookBreakdown.value.reduce((sum, item) => sum + item.duration, 0)
})
const trendTotalDuration = computed(() => {
  // 注意：年视图下 trendData 已经是月聚合数据，这里计算总时长依然正确
  return trendData.value.reduce((sum, item) => sum + item.duration, 0)
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
  if (h > 0) return `${h}h${m}m`
  if (m > 0) return `${m}m${s}s`
  return `${s}s`
}

function formatFullSeconds(duration: number): string {
  const safe = Math.max(0, Math.floor(duration))
  const h = Math.floor(safe / 3600)
  const m = Math.floor((safe % 3600) / 60)
  const s = safe % 60
  return [h, m, s].map((part) => String(part).padStart(2, '0')).join(':')
}

function trendHeight(duration: number): string {
  if (duration <= 0) return '0%'
  if (trendMaxDuration.value <= 0) return '0%'
  const percent = (duration / trendMaxDuration.value) * 100
  // 最小显示 4% 高度，避免看不见
  return `${Math.max(4, Math.min(100, percent))}%`
}

function formatTrendLabel(date: string, _index: number): string {
  if (periodType.value === 'year') {
    // 年视图：date 为 YYYY-MM
    return `${Number(date.slice(5, 7))}月`
  }
  if (periodType.value === 'month') {
    // 月视图：2026-03-04 -> 04 (只显示日)
    return date.slice(8)
  }
  // 周视图：2026-03-04 -> 03-04
  return date.slice(5)
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

async function showDateDetails(item: StudyStatsTrendItem) {
  if (item.duration <= 0) return
  if (periodType.value === 'year') return // 年视图点击暂不展开详情，或未来可实现按月展开

  detailDate.value = item.date
  detailModalVisible.value = true
  detailLoading.value = true
  try {
    detailSessions.value = await getStudySessionsByDate(item.date, buildFilters())
  } catch (error) {
    message.error(String(error))
    detailModalVisible.value = false
  } finally {
    detailLoading.value = false
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

function resetFilters() {
  selectedSeries.value = 'all'
  selectedBookId.value = 'all'
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
    <a-spin :spinning="loading" wrapper-class-name="h-full">
      <div class="stats-shell">
        <div class="stats-toolbar">
          <div class="toolbar-main">
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

            <div class="summary-chips">
              <span class="summary-chip"
                >{{ t('studyStats.duration') }} {{ formatFullSeconds(trendTotalDuration) }}</span
              >
              <span class="summary-chip"
                >{{ t('studyStats.recentSessions') }} {{ totalRecent }}</span
              >
            </div>
          </div>

          <div class="filters">
            <a-select v-model:value="selectedSeries" class="series-select">
              <a-select-option value="all">{{ t('studyStats.allSeries') }}</a-select-option>
              <a-select-option value="1">{{ t('studyStats.seriesVocabulary') }}</a-select-option>
              <a-select-option value="2">{{ t('studyStats.seriesGrammar') }}</a-select-option>
            </a-select>

            <a-select v-model:value="selectedBookId" class="book-select">
              <a-select-option value="all">{{ t('studyStats.allBooks') }}</a-select-option>
              <a-select-option
                v-for="book in filteredBookOptions"
                :key="book.id"
                :value="String(book.id)"
              >
                {{ book.title }}
              </a-select-option>
            </a-select>

            <a-button class="reset-btn" @click="resetFilters">
              {{ t('studyStats.resetFilters') }}
            </a-button>
          </div>
        </div>

        <div class="stats-grid" :class="{ 'is-year-view': periodType === 'year' }">
          <article class="stats-card trend-card">
            <header class="card-title">{{ t('studyStats.trend') }}</header>

            <div v-if="trendData.length === 0" class="empty-state">
              {{ t('studyStats.empty') }}
            </div>
            <div v-else class="trend-container">
              <div class="trend-y-axis">
                <span v-for="label in yAxisLabels" :key="label" class="y-label">{{ label }}</span>
              </div>
              <div class="trend-bars-wrapper">
                <div class="trend-bars">
                  <div
                    v-for="(item, index) in trendData"
                    :key="item.date"
                    class="trend-bar-item"
                    :class="{ clickable: item.duration > 0 }"
                    @click="showDateDetails(item)"
                  >
                    <div class="trend-bar-track">
                      <div
                        class="trend-bar-fill"
                        :style="{ height: trendHeight(item.duration) }"
                      ></div>
                    </div>
                    <div class="trend-bar-label">{{ formatTrendLabel(item.date, index) }}</div>
                  </div>
                </div>
              </div>
            </div>
          </article>

          <article class="stats-card">
            <header class="card-title">{{ t('studyStats.bookBreakdown') }}</header>
            <div v-if="bookBreakdown.length === 0" class="empty-state">
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
                <span class="breakdown-value">{{ formatFullSeconds(item.duration) }}</span>
              </li>
            </ul>
          </article>

          <article class="stats-card">
            <header class="card-title">{{ t('studyStats.seriesBreakdown') }}</header>
            <div v-if="seriesBreakdown.length === 0" class="empty-state">
              {{ t('studyStats.empty') }}
            </div>
            <ul v-else class="breakdown-list">
              <li v-for="item in seriesBreakdown" :key="item.seriesKey" class="breakdown-item">
                <span class="breakdown-name">{{ resolveSeriesLabel(item.seriesKey) }}</span>
                <span class="breakdown-value">{{ formatFullSeconds(item.duration) }}</span>
              </li>
            </ul>
          </article>
        </div>

        <article class="stats-card recent-card">
          <header class="card-title">{{ t('studyStats.recentSessions') }}</header>

          <div v-if="recentSessions.length === 0" class="empty-state">
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
                    <td>{{ formatFullSeconds(session.duration) }}</td>
                    <td>{{ formatIsoToLocalMinute(session.startAt) }}</td>
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
    </a-spin>

    <a-modal
      v-model:open="detailModalVisible"
      :title="detailDate + ' ' + t('studyStats.duration')"
      :footer="null"
      width="700px"
    >
      <a-spin :spinning="detailLoading">
        <div class="modal-content">
          <div v-if="detailSessions.length === 0" class="empty-state">
            {{ t('studyStats.empty') }}
          </div>
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
                <tr v-for="session in detailSessions" :key="session.id">
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
  </section>
</template>

<style scoped>
.study-stats-page {
  box-sizing: border-box;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  padding: 14px;
  overflow-y: auto;
  overflow-x: hidden;
}

.stats-shell {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  gap: 14px;
}

.stats-toolbar {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  gap: 12px;
  padding: 12px;
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 50%, transparent);
  border-radius: 12px;
  background: v-bind('token.colorBgContainer');
}

.toolbar-main {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
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
  transition: all 0.2s ease;
}

.period-btn.active {
  background: v-bind('token.colorPrimary');
  color: #fff;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.summary-chips {
  display: inline-flex;
  flex-wrap: wrap;
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
  display: flex;
  flex-wrap: wrap;
  flex: 1 1 auto;
  min-width: 0;
  gap: 8px;
  justify-content: flex-end;
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

.stats-grid {
  display: grid;
  grid-template-columns: 1.6fr 1fr 1fr;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  gap: 12px;
}

/* 年视图下，趋势图占据整行 */
.stats-grid.is-year-view .trend-card {
  grid-column: 1 / -1;
}

.stats-card {
  max-width: 100%;
  min-width: 0;
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 50%, transparent);
  border-radius: 12px;
  padding: 14px;
  background: v-bind('token.colorBgContainer');
}

.card-title {
  font-size: 15px;
  font-weight: 700;
  margin-bottom: 10px;
  color: v-bind('token.colorTextHeading');
}

.empty-state {
  min-height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: v-bind('token.colorTextSecondary');
}

.trend-container {
  display: flex;
  gap: 12px;
  min-height: 220px;
}

.trend-y-axis {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding-bottom: 30px; /* Offset for x-axis labels */
  font-size: 10px;
  color: v-bind('token.colorTextTertiary');
  text-align: right;
  min-width: 40px;
}

.trend-bars-wrapper {
  flex: 1;
  overflow-x: auto;
  overflow-y: hidden;
}

.trend-bars {
  display: flex;
  align-items: flex-end;
  gap: v-bind("periodType === 'year' ? '12px' : '2px'");
  height: 100%;
  width: 100%;
  min-width: min-content;
}

.trend-bar-item {
  flex: 1;
  min-width: v-bind("periodType === 'year' ? '40px' : (periodType === 'month' ? '18px' : '24px')");
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  transition: opacity 0.2s;
}

.trend-bar-item.clickable {
  cursor: pointer;
}

.trend-bar-item.clickable:hover {
  opacity: 0.8;
}

.trend-bar-track {
  width: 100%;
  height: 160px;
  border-radius: 6px;
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 30%, transparent);
  display: flex;
  align-items: flex-end;
  overflow: hidden;
}

.trend-bar-fill {
  width: 100%;
  border-radius: 6px;
  background: linear-gradient(to top, v-bind('token.colorPrimary'), v-bind('token.colorInfo'));
}

.trend-bar-label {
  font-size: 11px;
  color: v-bind('token.colorTextSecondary');
  white-space: nowrap;
}

.trend-bar-value {
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  color: v-bind('token.colorTextTertiary');
  white-space: nowrap;
}

.breakdown-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
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
  color: v-bind('token.colorText');
}

.breakdown-percent {
  color: v-bind('token.colorTextSecondary');
  font-variant-numeric: tabular-nums;
}

.breakdown-value {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
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
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.recent-table tbody tr:hover {
  background: color-mix(in srgb, v-bind('token.colorFillAlter') 30%, transparent);
}

.recent-pagination {
  margin-top: 14px;
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 12px;
}

.page-indicator {
  font-size: 12px;
  color: v-bind('token.colorTextSecondary');
}

.modal-content {
  min-height: 200px;
}

@media (max-width: 1100px) {
  .stats-grid {
    grid-template-columns: 1fr 1fr;
  }
  .stats-grid .trend-card {
    grid-column: 1 / -1;
  }
}

@media (max-width: 960px) {
  .study-stats-page {
    padding: 10px;
  }

  /* 窄屏下筛选栏反转，下拉框显示在上方 */
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

  .stats-grid {
    grid-template-columns: 1fr;
  }

  .summary-chip {
    font-size: 11px;
  }

  .trend-y-axis {
    display: none;
  }
}

@media (max-width: 480px) {
  .filters {
    flex-direction: column;
  }
  .series-select,
  .book-select,
  .reset-btn {
    width: 100%;
  }
}
</style>
