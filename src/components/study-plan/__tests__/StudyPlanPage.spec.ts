import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import StudyPlanPage from '../StudyPlanPage.vue'

const studyPlanPageState = vi.hoisted(() => ({
  viewMode: 'month' as 'week' | 'month',
  monthCellDates: [] as string[],
  weekDates: [] as string[],
  summaryOf: vi.fn(),
}))

vi.mock('../../../composables/study-plan/useStudyPlanPage', () => ({
  useStudyPlanPage: () => ({
    t: (key: string) => key,
    token: {},
    viewModes: ['week', 'month'],
    viewMode: studyPlanPageState.viewMode,
    setViewMode: vi.fn(),
    periodLabel: '2026-08',
    goPrev: vi.fn(),
    goNext: vi.fn(),
    goToday: vi.fn(),
    loadingSummary: false,
    weekdays: [],
    weekdayKeys: [],
    monthCellDates: studyPlanPageState.monthCellDates,
    weekDates: studyPlanPageState.weekDates,
    selectedDate: '2026-08-13',
    getDateCellStyle: vi.fn(),
    getDateDayNumber: vi.fn(),
    isTodayDate: vi.fn(() => false),
    isCurrentMonthDate: vi.fn(),
    summaryOf: studyPlanPageState.summaryOf,
    selectWeekDate: vi.fn(),
    openMonthDrawer: vi.fn(),
    groupedTasksByDate: {},
    isDateTasksLoading: vi.fn(() => false),
    completingTaskId: null,
    jumpToStudy: vi.fn(),
    markTaskDone: vi.fn(),
    drawerOpen: false,
    selectedDateLabel: '2026-08-13',
  }),
}))

/** 挂载学习计划页面并替换外部 UI 组件。 */
function mountStudyPlanPage() {
  return mount(StudyPlanPage, {
    global: {
      stubs: {
        'a-button': { template: '<button><slot name="icon" /><slot /></button>' },
        'a-drawer': { template: '<div><slot /></div>' },
        DailyTaskPanel: true,
      },
    },
  })
}

describe('StudyPlanPage', () => {
  beforeEach(() => {
    studyPlanPageState.viewMode = 'month'
    studyPlanPageState.monthCellDates = []
    studyPlanPageState.weekDates = []
    studyPlanPageState.summaryOf.mockReset()
  })

  it('月视图将时间导航和今天按钮保持在同一导航组', () => {
    const wrapper = mountStudyPlanPage()

    const navigation = wrapper.find('.period-nav')
    expect(navigation.classes()).toContain('is-month-view')
    expect(navigation.find('.period-label').text()).toBe('2026-08')
    expect(navigation.find('.today-btn').text()).toBe('studyPlan.nav.today')
  })

  it.each(['month', 'week'] as const)('%s 视图在逾期数量为 0 时隐藏逾期项', (viewMode) => {
    studyPlanPageState.viewMode = viewMode
    studyPlanPageState.monthCellDates = viewMode === 'month' ? ['2026-08-13'] : []
    studyPlanPageState.weekDates = viewMode === 'week' ? ['2026-08-13'] : []
    studyPlanPageState.summaryOf.mockReturnValue({
      date: '2026-08-13',
      total: 1,
      due: 1,
      overdue: 0,
      completed: 0,
    })

    const wrapper = mountStudyPlanPage()

    expect(wrapper.find('.stat-item.overdue').exists()).toBe(false)
  })

  it.each(['month', 'week'] as const)('%s 视图在存在逾期时显示逾期数量', (viewMode) => {
    studyPlanPageState.viewMode = viewMode
    studyPlanPageState.monthCellDates = viewMode === 'month' ? ['2026-08-13'] : []
    studyPlanPageState.weekDates = viewMode === 'week' ? ['2026-08-13'] : []
    studyPlanPageState.summaryOf.mockReturnValue({
      date: '2026-08-13',
      total: 3,
      due: 3,
      overdue: 2,
      completed: 0,
    })

    const overdue = mountStudyPlanPage().find('.stat-item.overdue')

    expect(overdue.text()).toContain('studyPlan.overdue')
    expect(overdue.find('.stat-value').text()).toBe('2')
  })
})
