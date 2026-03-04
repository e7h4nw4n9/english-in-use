<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { message, theme } from 'ant-design-vue'
import { LeftOutlined, RightOutlined } from '@ant-design/icons-vue'
import type { Book, StudyTaskItem, StudyTaskSummaryDay } from '../../types'
import { useAppStore } from '../../stores/app'
import { useReaderStore } from '../../stores/reader'
import { getBooks, updateReadingProgress } from '../../lib/api'
import {
  completeStudyTask,
  getStudyTasksByDate,
  getStudyTasksSummary,
} from '../../lib/api/studyPlan'
import DailyTaskPanel from './DailyTaskPanel.vue'
import type { GroupedSeriesTasks } from './taskGroups'

const { t } = useI18n()
const { useToken } = theme
const { token } = useToken()

const appStore = useAppStore()
const readerStore = useReaderStore()

type CalendarViewMode = 'week' | 'month'

const viewMode = ref<CalendarViewMode>('week')
const anchorDate = ref(formatDate(new Date()))
const selectedDate = ref(formatDate(new Date()))
const todayDate = ref(formatDate(new Date()))

const summaryByDate = ref<Record<string, StudyTaskSummaryDay>>({})
const loadingSummary = ref(false)
const completingTaskId = ref<number | null>(null)

const tasksByDateCache = ref<Record<string, StudyTaskItem[]>>({})
const loadingDateTasks = ref<Record<string, boolean>>({})

const drawerOpen = ref(false)

const booksByCode = ref<Record<string, Book>>({})
const viewModes: CalendarViewMode[] = ['week', 'month']

function formatDate(date: Date): string {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

function parseDate(dateStr: string): Date {
  return new Date(`${dateStr}T00:00:00`)
}

function addDays(date: Date, days: number): Date {
  const next = new Date(date)
  next.setDate(next.getDate() + days)
  return next
}

function addMonths(date: Date, months: number): Date {
  const next = new Date(date)
  next.setMonth(next.getMonth() + months)
  return next
}

function startOfWeek(date: Date): Date {
  const copy = new Date(date)
  const day = copy.getDay()
  const diff = day === 0 ? -6 : 1 - day
  copy.setDate(copy.getDate() + diff)
  copy.setHours(0, 0, 0, 0)
  return copy
}

function endOfWeek(date: Date): Date {
  return addDays(startOfWeek(date), 6)
}

function startOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), 1)
}

function endOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth() + 1, 0)
}

function getRange(mode: CalendarViewMode, anchor: Date): { start: string; end: string } {
  if (mode === 'week') {
    return {
      start: formatDate(startOfWeek(anchor)),
      end: formatDate(endOfWeek(anchor)),
    }
  }

  return {
    start: formatDate(startOfMonth(anchor)),
    end: formatDate(endOfMonth(anchor)),
  }
}

const activeRange = computed(() => getRange(viewMode.value, parseDate(anchorDate.value)))

const periodLabel = computed(() => {
  const anchor = parseDate(anchorDate.value)
  if (viewMode.value === 'week') return `${activeRange.value.start} ~ ${activeRange.value.end}`
  return `${anchor.getFullYear()}-${String(anchor.getMonth() + 1).padStart(2, '0')}`
})

const monthCellDates = computed(() => {
  const anchor = parseDate(anchorDate.value)
  const monthStart = startOfMonth(anchor)
  const monthEnd = endOfMonth(anchor)
  const gridStart = startOfWeek(monthStart)
  const gridEnd = endOfWeek(monthEnd)

  const dates: string[] = []
  let cursor = gridStart
  while (cursor <= gridEnd) {
    dates.push(formatDate(cursor))
    cursor = addDays(cursor, 1)
  }
  return dates
})

const weekDates = computed(() => {
  const start = startOfWeek(parseDate(anchorDate.value))
  return new Array(7).fill(0).map((_, index) => formatDate(addDays(start, index)))
})

const currentMonthKey = computed(() => anchorDate.value.slice(0, 7))

const weekdays = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

function summaryOf(date: string): StudyTaskSummaryDay | null {
  return summaryByDate.value[date] || null
}

function isCurrentMonthDate(date: string): boolean {
  return date.slice(0, 7) === currentMonthKey.value
}

function isTodayDate(date: string): boolean {
  return date === todayDate.value
}

function isDateTasksLoading(date: string): boolean {
  return Boolean(loadingDateTasks.value[date])
}

function getDateCellStyle(date: string, selected: boolean) {
  const isMuted = viewMode.value === 'month' && !isCurrentMonthDate(date)

  let backgroundColor = isMuted ? token.value.colorFillSecondary : token.value.colorFillTertiary
  let borderColor = 'transparent'

  if (selected) {
    backgroundColor = token.value.colorPrimaryBg
    borderColor = token.value.colorPrimary
  }

  return {
    borderColor,
    backgroundColor,
    borderWidth: selected ? '2px' : '1px',
    borderStyle: 'solid',
    color: isMuted ? token.value.colorTextTertiary : token.value.colorText,
  }
}

function getSeriesMeta(book?: Book): { key: string; name: string; order: number } {
  if (!book) {
    return {
      key: 'unknown',
      name: t('studyPlan.unknownSeries'),
      order: 99,
    }
  }

  if (book.book_group === 1) {
    return {
      key: 'vocabulary',
      name: t('app.bookGroups.vocabulary'),
      order: 1,
    }
  }

  if (book.book_group === 2) {
    return {
      key: 'grammar',
      name: t('app.bookGroups.grammar'),
      order: 2,
    }
  }

  return {
    key: `group-${book.book_group}`,
    name: t('studyPlan.unknownSeries'),
    order: 90,
  }
}

function groupTasksBySeries(tasks: StudyTaskItem[]): GroupedSeriesTasks[] {
  const seriesMap = new Map<
    string,
    {
      order: number
      seriesName: string
      booksMap: Map<string, { bookCode: string; bookTitle: string; tasks: StudyTaskItem[] }>
      total: number
    }
  >()

  for (const task of tasks) {
    const book = booksByCode.value[task.productCode]
    const { key, name, order } = getSeriesMeta(book)

    if (!seriesMap.has(key)) {
      seriesMap.set(key, {
        order,
        seriesName: name,
        booksMap: new Map(),
        total: 0,
      })
    }

    const seriesEntry = seriesMap.get(key)
    if (!seriesEntry) continue

    const bookCode = task.productCode || 'unknown'
    const bookTitle = book?.title || t('studyPlan.unknownBook')

    if (!seriesEntry.booksMap.has(bookCode)) {
      seriesEntry.booksMap.set(bookCode, {
        bookCode,
        bookTitle,
        tasks: [],
      })
    }

    const bookEntry = seriesEntry.booksMap.get(bookCode)
    if (!bookEntry) continue

    bookEntry.tasks.push(task)
    seriesEntry.total += 1
  }

  return Array.from(seriesMap.entries())
    .sort((a, b) => a[1].order - b[1].order)
    .map(([seriesKey, series]) => ({
      seriesKey,
      seriesName: series.seriesName,
      total: series.total,
      books: Array.from(series.booksMap.values())
        .map((book) => ({
          ...book,
          tasks: book.tasks.slice().sort((a, b) => {
            if (a.scheduledDate !== b.scheduledDate)
              return a.scheduledDate.localeCompare(b.scheduledDate)
            if (a.reviewStage !== b.reviewStage) return a.reviewStage - b.reviewStage
            return a.taskId - b.taskId
          }),
        }))
        .sort((a, b) => a.bookTitle.localeCompare(b.bookTitle)),
    }))
}

const groupedTasksByDate = computed(() => {
  const grouped: Record<string, GroupedSeriesTasks[]> = {}
  for (const [date, tasks] of Object.entries(tasksByDateCache.value)) {
    grouped[date] = groupTasksBySeries(tasks)
  }
  return grouped
})

function setViewMode(mode: CalendarViewMode) {
  if (mode === viewMode.value) return
  viewMode.value = mode
  const { start, end } = getRange(mode, parseDate(anchorDate.value))

  // 如果当前范围内包含今天，则默认选择今天
  if (todayDate.value >= start && todayDate.value <= end) {
    selectedDate.value = todayDate.value
  } else {
    selectedDate.value = start
  }

  drawerOpen.value = false
}

function goPrev() {
  const anchor = parseDate(anchorDate.value)
  if (viewMode.value === 'week') {
    anchorDate.value = formatDate(addDays(anchor, -7))
  } else {
    anchorDate.value = formatDate(addMonths(anchor, -1))
  }

  selectedDate.value = getRange(viewMode.value, parseDate(anchorDate.value)).start
  drawerOpen.value = false
}

function goNext() {
  const anchor = parseDate(anchorDate.value)
  if (viewMode.value === 'week') {
    anchorDate.value = formatDate(addDays(anchor, 7))
  } else {
    anchorDate.value = formatDate(addMonths(anchor, 1))
  }

  selectedDate.value = getRange(viewMode.value, parseDate(anchorDate.value)).start
  drawerOpen.value = false
}

function goToday() {
  const today = formatDate(new Date())
  anchorDate.value = today
  selectedDate.value = today
  drawerOpen.value = false
}

function selectWeekDate(date: string) {
  selectedDate.value = date
}

function openMonthDrawer(date: string) {
  selectedDate.value = date
  drawerOpen.value = true
  ensureDateTasks(date)
}

async function refreshSummary() {
  loadingSummary.value = true
  try {
    const response = await getStudyTasksSummary(
      activeRange.value.start,
      activeRange.value.end,
      viewMode.value,
    )
    const summaryMap: Record<string, StudyTaskSummaryDay> = {}
    for (const day of response.days) {
      summaryMap[day.date] = day
    }
    summaryByDate.value = summaryMap
  } catch (error) {
    const errorText = error instanceof Error ? error.message : String(error)
    message.error(t('studyPlan.loadFailed', { error: errorText }))
  } finally {
    loadingSummary.value = false
  }
}

async function ensureDateTasks(date: string, force = false) {
  if (!force && tasksByDateCache.value[date]) return

  loadingDateTasks.value = {
    ...loadingDateTasks.value,
    [date]: true,
  }

  try {
    const dateTasks = await getStudyTasksByDate(date)
    tasksByDateCache.value = {
      ...tasksByDateCache.value,
      [date]: dateTasks,
    }
  } catch (error) {
    const errorText = error instanceof Error ? error.message : String(error)
    message.error(t('studyPlan.loadFailed', { error: errorText }))
  } finally {
    loadingDateTasks.value = {
      ...loadingDateTasks.value,
      [date]: false,
    }
  }
}

async function markTaskDone(task: StudyTaskItem) {
  if (task.taskStatus === 1 || completingTaskId.value !== null) return

  completingTaskId.value = task.taskId
  try {
    await completeStudyTask(task.taskId)
    await Promise.all([
      refreshSummary(),
      ensureDateTasks(task.scheduledDate, true),
      ensureDateTasks(selectedDate.value, true),
    ])
    message.success(t('studyPlan.completed'))
  } catch (error) {
    const errorText = error instanceof Error ? error.message : String(error)
    message.error(t('studyPlan.actionFailed', { error: errorText }))
  } finally {
    completingTaskId.value = null
  }
}

async function jumpToStudy(task: StudyTaskItem) {
  const book = booksByCode.value[task.productCode]
  if (!book) {
    message.error(t('studyPlan.bookNotFound'))
    return
  }

  try {
    await updateReadingProgress(task.productCode, task.resourceId || null, null, 1, 0, 0)
  } catch {
    // Keep navigation responsive even if progress write fails.
  }

  readerStore.pendingStudyResourceId = task.resourceId || null
  readerStore.currentUnitName = task.unitName
  appStore.currentBook = book
}

watch(
  [viewMode, anchorDate],
  () => {
    refreshSummary()
  },
  { immediate: true },
)

watch(
  selectedDate,
  () => {
    ensureDateTasks(selectedDate.value)
  },
  { immediate: true },
)

onMounted(async () => {
  try {
    const books = await getBooks()
    const map: Record<string, Book> = {}
    for (const book of books) {
      map[book.product_code] = book
    }
    booksByCode.value = map
  } catch {
    booksByCode.value = {}
  }
})
</script>

<template>
  <section class="study-plan-page h-full w-full">
    <div class="study-plan-shell">
      <div class="study-card study-toolbar">
        <div class="mode-switch" role="tablist" :aria-label="t('app.homeTabs.studyPlan')">
          <button
            v-for="mode in viewModes"
            :key="mode"
            type="button"
            class="mode-btn"
            :class="{ active: mode === viewMode }"
            :aria-selected="mode === viewMode"
            @click="setViewMode(mode)"
          >
            {{ t(`studyPlan.view.${mode}`) }}
          </button>
        </div>

        <div class="period-nav">
          <a-button shape="circle" aria-label="Previous period" @click="goPrev">
            <template #icon><LeftOutlined /></template>
          </a-button>
          <div class="period-label">{{ periodLabel }}</div>
          <a-button shape="circle" aria-label="Next period" @click="goNext">
            <template #icon><RightOutlined /></template>
          </a-button>
          <a-button type="default" class="today-btn" @click="goToday">
            {{ t('studyPlan.nav.today') }}
          </a-button>
        </div>
      </div>

      <div class="study-card study-grid">
        <div v-if="loadingSummary" class="loading-state">
          {{ t('app.loading') }}
        </div>

        <template v-else>
          <template v-if="viewMode === 'month'">
            <div class="weekday-row">
              <div v-for="day in weekdays" :key="`wk-${day}`" class="weekday-cell">{{ day }}</div>
            </div>
            <div class="date-grid-month">
              <button
                v-for="date in monthCellDates"
                :key="date"
                type="button"
                class="date-cell"
                :class="{
                  'is-selected': selectedDate === date,
                  'is-muted': !isCurrentMonthDate(date),
                }"
                :style="getDateCellStyle(date, selectedDate === date)"
                @click="openMonthDrawer(date)"
              >
                <div class="date-cell-head">
                  <span class="date-number">{{ date.slice(8) }}</span>
                  <span v-if="isTodayDate(date)" class="today-tag">
                    {{ t('studyPlan.nav.today') }}
                  </span>
                </div>
                <div class="date-badge-group">
                  <span class="date-badge badge-completed">
                    {{ t('studyPlan.done') }} {{ summaryOf(date)?.completed || 0 }}
                  </span>
                  <span class="date-badge badge-overdue">
                    {{ t('studyPlan.overdue') }} {{ summaryOf(date)?.overdue || 0 }}
                  </span>
                  <span class="date-badge badge-total">
                    {{ t('studyPlan.total') }} {{ summaryOf(date)?.total || 0 }}
                  </span>
                </div>
              </button>
            </div>
          </template>

          <template v-else>
            <div class="date-grid-week">
              <button
                v-for="date in weekDates"
                :key="date"
                type="button"
                class="date-cell"
                :class="{ 'is-selected': selectedDate === date }"
                :style="getDateCellStyle(date, selectedDate === date)"
                @click="selectWeekDate(date)"
              >
                <div class="date-cell-head">
                  <span class="date-number">{{ date }}</span>
                  <span v-if="isTodayDate(date)" class="today-tag">
                    {{ t('studyPlan.nav.today') }}
                  </span>
                </div>
                <div class="date-badge-group">
                  <span class="date-badge badge-completed">
                    {{ t('studyPlan.done') }} {{ summaryOf(date)?.completed || 0 }}
                  </span>
                  <span class="date-badge badge-overdue">
                    {{ t('studyPlan.overdue') }} {{ summaryOf(date)?.overdue || 0 }}
                  </span>
                  <span class="date-badge badge-total">
                    {{ t('studyPlan.total') }} {{ summaryOf(date)?.total || 0 }}
                  </span>
                </div>
              </button>
            </div>
          </template>
        </template>
      </div>

      <div v-if="viewMode === 'week'" class="study-card week-task-panel">
        <div class="week-panel-head">{{ t('studyPlan.taskListFor', { date: selectedDate }) }}</div>
        <DailyTaskPanel
          :grouped-tasks="groupedTasksByDate[selectedDate] || []"
          :loading="isDateTasksLoading(selectedDate)"
          :show-actions="isTodayDate(selectedDate)"
          :completing-task-id="completingTaskId"
          @jump-to-study="jumpToStudy"
          @mark-task-done="markTaskDone"
        />
      </div>

      <a-drawer
        v-model:open="drawerOpen"
        :title="t('studyPlan.taskListFor', { date: selectedDate })"
        placement="bottom"
        :height="'70vh'"
        destroy-on-close
      >
        <DailyTaskPanel
          :grouped-tasks="groupedTasksByDate[selectedDate] || []"
          :loading="isDateTasksLoading(selectedDate)"
          :show-actions="isTodayDate(selectedDate)"
          :completing-task-id="completingTaskId"
          @jump-to-study="jumpToStudy"
          @mark-task-done="markTaskDone"
        />
      </a-drawer>
    </div>
  </section>
</template>

<style scoped>
.study-plan-page {
  position: relative;
  isolation: isolate;
  overflow-y: auto;
  min-height: 100%;
  padding: 14px;
}

.study-plan-shell {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 100%;
}

.study-card {
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 75%, transparent);
  border-radius: 16px;
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, v-bind('token.colorBgContainer') 96%, #ffffff 4%),
      color-mix(in srgb, v-bind('token.colorFillAlter') 62%, #ffffff 38%)
    ),
    v-bind('token.colorBgContainer');
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, #ffffff 60%, transparent),
    0 16px 34px -30px color-mix(in srgb, v-bind('token.colorText') 45%, transparent);
}

.study-toolbar {
  padding: 12px 14px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.mode-switch {
  display: inline-flex;
  gap: 4px;
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 75%, transparent);
  border-radius: 999px;
  padding: 4px;
  background: color-mix(in srgb, v-bind('token.colorBgElevated') 88%, transparent);
}

.mode-btn {
  border: 0;
  background: transparent;
  color: v-bind('token.colorTextSecondary');
  font-size: 12px;
  font-weight: 600;
  line-height: 1;
  padding: 8px 12px;
  border-radius: 999px;
  cursor: pointer;
  transition:
    color 0.2s ease,
    background-color 0.2s ease;
}

.mode-btn:hover {
  color: v-bind('token.colorText');
}

.mode-btn.active {
  color: v-bind('token.colorPrimary');
  background: color-mix(in srgb, v-bind('token.colorPrimaryBg') 85%, transparent);
}

.mode-btn:focus-visible {
  outline: 2px solid v-bind('token.colorPrimary');
  outline-offset: 2px;
}

.period-nav {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.period-label {
  min-width: 220px;
  text-align: center;
  font-size: 14px;
  font-weight: 700;
  color: v-bind('token.colorText');
}

.today-btn {
  flex-shrink: 0;
}

.study-grid {
  padding: 14px;
}

.weekday-row {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 8px;
  margin-bottom: 8px;
}

.weekday-cell {
  text-align: center;
  font-size: 11px;
  line-height: 1.3;
  font-weight: 700;
  color: v-bind('token.colorTextTertiary');
}

.date-grid-month,
.date-grid-week {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 8px;
}

.date-cell {
  cursor: pointer;
  position: relative;
  border: 1px solid transparent;
  border-radius: 12px;
  padding: 8px;
  text-align: left;
  min-height: 92px;
  width: 100%;
  transition:
    border-color 0.2s ease,
    box-shadow 0.2s ease,
    transform 0.2s ease;
}

.date-cell:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, v-bind('token.colorPrimaryBorder') 78%, transparent);
  box-shadow: 0 10px 16px -16px color-mix(in srgb, v-bind('token.colorPrimary') 40%, transparent);
}

.date-cell.is-selected {
  box-shadow: 0 0 0 1px color-mix(in srgb, v-bind('token.colorPrimary') 30%, transparent);
}

.date-cell.is-muted .date-number {
  opacity: 0.7;
}

.date-cell-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 6px;
  margin-bottom: 12px;
}

.date-number {
  font-size: 12px;
  line-height: 1.2;
  font-weight: 700;
}

.today-tag {
  position: absolute;
  top: 0;
  right: 0;
  font-size: 8px;
  font-weight: 800;
  color: #ffffff;
  background: v-bind('token.colorInfo');
  padding: 2px 6px;
  border-radius: 0 12px 0 12px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  box-shadow: -2px 2px 6px color-mix(in srgb, v-bind('token.colorInfo') 20%, transparent);
  z-index: 1;
}

.date-badge-group {
  position: absolute;
  left: 8px;
  right: 8px;
  bottom: 8px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}

.date-badge {
  font-size: 10px;
  line-height: 1;
  font-weight: 600;
  padding: 4px 6px;
  border-radius: 999px;
}

.badge-completed {
  color: v-bind('token.colorSuccess');
  background: color-mix(in srgb, v-bind('token.colorSuccessBg') 80%, transparent);
}

.badge-overdue {
  color: v-bind('token.colorError');
  background: color-mix(in srgb, v-bind('token.colorErrorBg') 82%, transparent);
}

.badge-total {
  color: v-bind('token.colorPrimary');
  background: color-mix(in srgb, v-bind('token.colorPrimaryBg') 84%, transparent);
}

.week-task-panel {
  padding: 14px;
}

.week-panel-head {
  margin-bottom: 10px;
  font-size: 14px;
  line-height: 1.3;
  font-weight: 700;
  color: v-bind('token.colorTextHeading');
}

.loading-state {
  padding: 20px 0;
  text-align: center;
  font-size: 13px;
  line-height: 1.4;
  color: v-bind('token.colorTextSecondary');
}

@media (max-width: 1024px) {
  .date-grid-month,
  .date-grid-week,
  .weekday-row {
    gap: 6px;
  }
}

@media (max-width: 768px) {
  .study-plan-page {
    padding: 10px;
  }

  .period-label {
    min-width: 150px;
  }

  .date-grid-month,
  .date-grid-week,
  .weekday-row {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .weekday-row {
    display: none;
  }
}

@media (prefers-reduced-motion: reduce) {
  .mode-btn,
  .date-cell {
    transition: none;
  }
}
</style>
