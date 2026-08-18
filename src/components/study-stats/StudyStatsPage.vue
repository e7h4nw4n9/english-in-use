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
} from '@/types'
import { getBooks } from '@/lib/api'
import { getStudySessionsByDate, getStudyStats } from '@/lib/api/studyTimer'
import { formatIsoToLocalMinute, generateDateRange, generateMonthRange } from '@/lib/datetime'
import { useAppStore } from '@/stores/app'
import StudyStatsDetailModal from './StudyStatsDetailModal.vue'
import StudyStatsToolbar from './StudyStatsToolbar.vue'
import StudyTrendCard from './StudyTrendCard.vue'

const { t } = useI18n()
const { useToken } = theme
const { token } = useToken()
const appStore = useAppStore()

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
let statsRequestVersion = 0

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

const totalDuration = computed(() => {
  return bookBreakdown.value.reduce((sum, item) => sum + item.duration, 0)
})
const trendTotalDuration = computed(() => {
  // 注意：年视图下 trendData 已经是月聚合数据，这里计算总时长依然正确
  return trendData.value.reduce((sum, item) => sum + item.duration, 0)
})

const totalRecent = computed(() => stats.value?.totalRecent || 0)
const totalPages = computed(() => Math.max(1, Math.ceil(totalRecent.value / pageSize.value)))

function formatFullSeconds(duration: number): string {
  const safe = Math.max(0, Math.floor(duration))
  const h = Math.floor(safe / 3600)
  const m = Math.floor((safe % 3600) / 60)
  const s = safe % 60
  return [h, m, s].map((part) => String(part).padStart(2, '0')).join(':')
}

function resolveSeriesLabel(seriesKey: string): string {
  if (seriesKey === 'vocabulary') return t('studyStats.seriesVocabulary')
  if (seriesKey === 'grammar') return t('studyStats.seriesGrammar')
  return t('studyStats.seriesOther')
}

/** 根据当前界面筛选项构造后端统计过滤条件。 */
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

/** 刷新当前周期和分页下的学习统计。 */
async function refreshStats() {
  const requestVersion = ++statsRequestVersion
  loading.value = true
  try {
    const response = await getStudyStats(
      periodType.value,
      buildFilters(),
      page.value,
      pageSize.value,
    )
    if (requestVersion === statsRequestVersion) {
      stats.value = response
    }
  } catch (error) {
    if (requestVersion !== statsRequestVersion) return
    const errorText = error instanceof Error ? error.message : String(error)
    message.error(t('studyStats.loadFailed', { error: errorText }))
  } finally {
    if (requestVersion === statsRequestVersion) {
      loading.value = false
    }
  }
}

/** 加载并展示趋势日期对应的会话明细。
 * @param item - 被选择的趋势数据点。
 */
async function showDateDetails(item: StudyStatsTrendItem) {
  if (item.duration <= 0) return
  if (periodType.value === 'year') return // 年视图点击暂不展开详情，或未来可实现按月展开
  if (detailLoading.value) return

  detailLoading.value = true
  try {
    await appStore.runGlobalLoadingAction(async () => {
      detailDate.value = item.date
      detailModalVisible.value = true
      detailSessions.value = await getStudySessionsByDate(item.date, buildFilters())
    }, t('studyStats.loadingDetails'))
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

watch(selectedSeries, () => {
  if (selectedBookId.value === 'all') return
  const group = Number(selectedSeries.value)
  const existing = books.value.some(
    (book) => book.book_group === group && String(book.id) === selectedBookId.value,
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
        <StudyStatsToolbar
          v-model:period-type="periodType"
          v-model:selected-series="selectedSeries"
          v-model:selected-book-id="selectedBookId"
          :books="books"
          :duration-text="formatFullSeconds(trendTotalDuration)"
          :total-recent="totalRecent"
        />

        <div
          class="stats-grid"
          :class="{ 'is-wide-trend-view': periodType === 'month' || periodType === 'year' }"
        >
          <StudyTrendCard
            class="trend-card"
            :period-type="periodType"
            :trend="trendData"
            @select="showDateDetails"
          />

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

    <StudyStatsDetailModal
      v-model:open="detailModalVisible"
      :loading="detailLoading"
      :date="detailDate"
      :sessions="detailSessions"
    />
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

.stats-grid {
  display: grid;
  grid-template-columns: 1.6fr 1fr 1fr;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  gap: 12px;
}

/* 月、年视图下，趋势图占据整行。 */
.stats-grid.is-wide-trend-view .trend-card {
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

  .stats-grid {
    grid-template-columns: 1fr;
  }
}
</style>
