import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { message, theme } from 'ant-design-vue'
import type { Book, StudyTaskItem, StudyTaskSummaryDay } from '../../types'
import { useAppStore } from '../../stores/app'
import { useReaderStore } from '../../stores/reader'
import { getBooks, updateReadingProgress } from '../../lib/api'
import {
  completeStudyTask,
  getStudyTasksByDate,
  getStudyTasksSummary,
} from '../../lib/api/studyPlan'
import { groupTasksBySeries, type GroupedSeriesTasks } from '../../components/study-plan/taskGroups'
import {
  addDays,
  addMonths,
  endOfMonth,
  endOfWeek,
  formatDate,
  getRange,
  parseDate,
  startOfMonth,
  startOfWeek,
  type CalendarViewMode,
} from './studyPlanCalendar'

/** 管理学习计划日历、任务缓存和阅读器跳转。 */
export function useStudyPlanPage() {
  const { t } = useI18n()
  const { useToken } = theme
  const { token } = useToken()

  const appStore = useAppStore()
  const readerStore = useReaderStore()

  const viewMode = ref<CalendarViewMode>('week')
  const anchorDate = ref(formatDate(new Date()))
  const selectedDate = ref(formatDate(new Date()))
  const todayDate = ref(formatDate(new Date()))

  const summaryByDate = ref<Record<string, StudyTaskSummaryDay>>({})
  const loadingSummary = ref(false)
  const completingTaskId = ref<number | null>(null)
  const openingTask = ref(false)

  const tasksByDateCache = ref<Record<string, StudyTaskItem[]>>({})
  const loadingDateTasks = ref<Record<string, boolean>>({})
  let summaryRequestVersion = 0
  const dateTaskRequestVersions = new Map<string, number>()

  const drawerOpen = ref(false)

  const booksByCode = ref<Record<string, Book>>({})
  const viewModes: CalendarViewMode[] = ['week', 'month']

  /** 在应用重新可见时刷新“今天”，避免跨午夜后日期状态过期。 */
  function refreshTodayDate() {
    todayDate.value = formatDate(new Date())
  }

  function handleVisibilityChange() {
    if (document.visibilityState === 'visible') {
      refreshTodayDate()
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

  const weekdayKeys = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'] as const
  const weekdays = computed(() => weekdayKeys.map((key) => t(`studyPlan.weekdayShort.${key}`)))

  function getDateDayNumber(date: string): string {
    return String(Number(date.slice(8)))
  }

  function getWeekdayLabel(date: string): string {
    const day = parseDate(date).getDay()
    const mondayFirstIndex = day === 0 ? 6 : day - 1
    return weekdays.value[mondayFirstIndex]
  }

  const selectedDateLabel = computed(() => {
    if (viewMode.value === 'week') {
      return `${selectedDate.value} · ${getWeekdayLabel(selectedDate.value)}`
    }
    return selectedDate.value
  })

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

    let backgroundColor = isMuted ? 'transparent' : token.value.colorFillQuaternary
    let borderColor = isMuted ? 'transparent' : token.value.colorBorderSecondary

    if (selected) {
      backgroundColor = `color-mix(in srgb, ${token.value.colorPrimaryBg}, transparent 20%)`
      borderColor = token.value.colorPrimary
    }

    return {
      borderColor,
      backgroundColor,
      borderWidth: selected ? '2px' : '1px',
      borderStyle: 'solid',
      color: isMuted ? token.value.colorTextTertiary : token.value.colorText,
      opacity: isMuted ? 0.4 : 1,
    }
  }

  const groupedTasksByDate = computed(() => {
    const grouped: Record<string, GroupedSeriesTasks[]> = {}
    for (const [date, tasks] of Object.entries(tasksByDateCache.value)) {
      grouped[date] = groupTasksBySeries(tasks, booksByCode.value, t)
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

  /** 刷新当前日历范围的任务汇总，并丢弃过期请求结果。 */
  async function refreshSummary() {
    const requestVersion = ++summaryRequestVersion
    loadingSummary.value = true
    try {
      const response = await getStudyTasksSummary(
        activeRange.value.start,
        activeRange.value.end,
        viewMode.value,
      )
      if (requestVersion !== summaryRequestVersion) return
      const summaryMap: Record<string, StudyTaskSummaryDay> = {}
      for (const day of response.days) {
        summaryMap[day.date] = day
      }
      summaryByDate.value = summaryMap
    } catch (error) {
      if (requestVersion !== summaryRequestVersion) return
      const errorText = error instanceof Error ? error.message : String(error)
      message.error(t('studyPlan.loadFailed', { error: errorText }))
    } finally {
      if (requestVersion === summaryRequestVersion) {
        loadingSummary.value = false
      }
    }
  }

  /** 按需加载指定日期任务，并使用版本号避免旧请求覆盖。
   * @param date - 目标日期。
   * @param force - 是否忽略现有缓存强制刷新。
   */
  async function ensureDateTasks(date: string, force = false) {
    if (!force && tasksByDateCache.value[date]) return
    const requestVersion = (dateTaskRequestVersions.get(date) ?? 0) + 1
    dateTaskRequestVersions.set(date, requestVersion)

    loadingDateTasks.value = {
      ...loadingDateTasks.value,
      [date]: true,
    }

    try {
      const dateTasks = await getStudyTasksByDate(date)
      if (dateTaskRequestVersions.get(date) !== requestVersion) return
      tasksByDateCache.value = {
        ...tasksByDateCache.value,
        [date]: dateTasks,
      }
    } catch (error) {
      if (dateTaskRequestVersions.get(date) !== requestVersion) return
      const errorText = error instanceof Error ? error.message : String(error)
      message.error(t('studyPlan.loadFailed', { error: errorText }))
    } finally {
      if (dateTaskRequestVersions.get(date) === requestVersion) {
        loadingDateTasks.value = {
          ...loadingDateTasks.value,
          [date]: false,
        }
      }
    }
  }

  /** 完成任务并刷新相关日期与汇总状态。
   * @param task - 需要完成的学习任务。
   */
  async function markTaskDone(task: StudyTaskItem) {
    if (task.taskStatus === 1 || completingTaskId.value !== null) return

    completingTaskId.value = task.taskId
    try {
      await appStore.runGlobalLoadingAction(async () => {
        await completeStudyTask(task.taskId)
        const datesToRefresh = new Set([task.scheduledDate, selectedDate.value])
        await Promise.all([
          refreshSummary(),
          ...Array.from(datesToRefresh, (date) => ensureDateTasks(date, true)),
        ])
        message.success(t('studyPlan.completed'))
      }, t('studyPlan.completingTask'))
    } catch (error) {
      const errorText = error instanceof Error ? error.message : String(error)
      message.error(t('studyPlan.actionFailed', { error: errorText }))
    } finally {
      completingTaskId.value = null
    }
  }

  /** 将任务目标写入阅读器状态并跳转到对应图书。
   * @param task - 目标学习任务。
   */
  async function jumpToStudy(task: StudyTaskItem) {
    if (openingTask.value) return
    const book = booksByCode.value[task.productCode]
    if (!book) {
      message.error(t('studyPlan.bookNotFound'))
      return
    }

    openingTask.value = true
    try {
      await appStore.runGlobalLoadingAction(async () => {
        try {
          await updateReadingProgress(task.productCode, task.resourceId || null, null, 1, 0, 0)
        } catch {
          // 即使进度写入失败，也继续保持页面跳转可用。
        }

        readerStore.pendingStudyResourceId = task.resourceId || null
        readerStore.currentUnitName = task.unitName
        appStore.currentBook = book
      }, t('studyPlan.openingTask'))
    } finally {
      openingTask.value = false
    }
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
    refreshTodayDate()
    document.addEventListener('visibilitychange', handleVisibilityChange)
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

  onBeforeUnmount(() => {
    document.removeEventListener('visibilitychange', handleVisibilityChange)
  })

  return {
    t,
    token,
    viewModes,
    viewMode,
    setViewMode,
    periodLabel,
    goPrev,
    goNext,
    goToday,
    loadingSummary,
    weekdays,
    weekdayKeys,
    monthCellDates,
    weekDates,
    selectedDate,
    currentMonthKey,
    todayDate,
    getDateCellStyle,
    getDateDayNumber,
    isTodayDate,
    isCurrentMonthDate,
    summaryOf,
    selectWeekDate,
    openMonthDrawer,
    groupedTasksByDate,
    isDateTasksLoading,
    completingTaskId,
    jumpToStudy,
    markTaskDone,
    drawerOpen,
    selectedDateLabel,
  }
}
