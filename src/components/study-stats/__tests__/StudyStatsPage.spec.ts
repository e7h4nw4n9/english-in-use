import { flushPromises, mount } from '@vue/test-utils'
import { Select, SelectOption } from 'ant-design-vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import StudyStatsPage from '../StudyStatsPage.vue'
import StudyListFilters from '../StudyListFilters.vue'
import StudyStatsToolbar from '../StudyStatsToolbar.vue'
import { getBooks } from '../../../lib/api'
import {
  getStudySessionsByDate,
  getStudyStats,
  getStudySessionPage,
} from '../../../lib/api/studyTimer'

vi.mock('../../../lib/api', () => ({
  getBooks: vi.fn(),
}))

vi.mock('../../../lib/api/studyTimer', () => ({
  deleteStudySession: vi.fn(),
  getStudySessionsByDate: vi.fn(),
  getStudyStats: vi.fn(),
  getStudySessionPage: vi.fn(),
  updateStudySession: vi.fn(),
}))

vi.mock('../../../stores/app', () => ({
  useAppStore: () => ({
    runGlobalLoadingAction: vi.fn(async (action: () => Promise<unknown>) => action()),
  }),
}))

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

const passthroughStub = { template: '<div><slot /></div>' }

describe('StudyStatsPage', () => {
  it('三个同级页签共享页面，趋势与列表筛选相互独立', async () => {
    const wrapper = mount(StudyStatsPage, {
      global: {
        stubs: {
          'a-spin': passthroughStub,
          'a-select': passthroughStub,
          'a-select-option': passthroughStub,
          'a-button': passthroughStub,
          'a-modal': passthroughStub,
          StudyArrangements: {
            name: 'StudyArrangements',
            props: ['active', 'filters', 'status'],
            template: '<div />',
          },
        },
      },
    })
    await flushPromises()
    const tabs = wrapper.findComponent({ name: 'ATabs' })
    expect(tabs.props('activeKey')).toBe('trend')
    expect(tabs.props('centered')).toBe(true)
    expect(wrapper.findAllComponents({ name: 'ATabPane' }).map((tab) => tab.props('tab'))).toEqual([
      'studyStats.trendOverview',
      'studyStats.recordDetails',
      'studyStats.unitReviewPlans',
    ])
    expect(wrapper.find('.trend-section').exists()).toBe(true)
    expect(wrapper.find('.trend-section').classes()).not.toContain('ant-card')
    expect(wrapper.find('.records-section').exists()).toBe(true)
    expect(wrapper.find('.stats-divider').exists()).toBe(false)
    expect(wrapper.find('.records-section').classes()).not.toContain('ant-card')
    const lower = wrapper.findComponent(StudyListFilters)
    const upper = wrapper.findComponent(StudyStatsToolbar)
    const statsCalls = vi.mocked(getStudyStats).mock.calls.length
    tabs.vm.$emit('update:activeKey', 'records')
    await flushPromises()
    expect(lower.props('showDates')).toBe(true)
    expect(lower.props('showStatus')).toBe(false)
    lower.vm.$emit('update:series', '1')
    lower.vm.$emit('update:bookId', '1')
    lower.vm.$emit('update:dateRange', ['2026-08-01', '2026-08-31'])
    await flushPromises()
    expect(getStudyStats).toHaveBeenCalledTimes(statsCalls)
    expect(getStudySessionPage).toHaveBeenLastCalledWith(
      expect.objectContaining({ bookId: 1, bookGroup: 1, rangeStart: '2026-08-01' }),
    )
    const recordCalls = vi.mocked(getStudySessionPage).mock.calls.length
    upper.vm.$emit('update:periodType', 'year')
    await flushPromises()
    expect(getStudySessionPage).toHaveBeenCalledTimes(recordCalls)
    tabs.vm.$emit('update:activeKey', 'arrangements')
    await flushPromises()
    expect(lower.props('showDates')).toBe(false)
    expect(lower.props('showStatus')).toBe(true)
    expect(lower.find('.operation-button').exists()).toBe(false)
    expect(wrapper.findComponent({ name: 'StudyArrangements' }).props('filters')).toEqual({
      bookId: 1,
      bookGroup: 1,
    })
    wrapper.unmount()
  })
  it('默认显示学习趋势，记录和复习计划按进入页签激活', async () => {
    const wrapper = mount(StudyStatsPage, {
      global: {
        stubs: {
          'a-spin': passthroughStub,
          'a-select': passthroughStub,
          'a-select-option': passthroughStub,
          'a-button': passthroughStub,
          'a-modal': passthroughStub,
          StudyArrangements: {
            name: 'StudyArrangements',
            props: ['active', 'filters', 'status'],
            template: '<div class="arrangements-stub" />',
          },
        },
      },
    })
    await flushPromises()
    expect(wrapper.find('.arrangements-stub').exists()).toBe(false)
    expect(wrapper.findComponent({ name: 'ATabs' }).props('centered')).toBe(true)
    expect(getStudySessionPage).not.toHaveBeenCalled()
    wrapper.findComponent({ name: 'ATabs' }).vm.$emit('update:activeKey', 'records')
    await flushPromises()
    expect(getStudySessionPage).toHaveBeenCalledOnce()
    wrapper.findComponent({ name: 'ATabs' }).vm.$emit('update:activeKey', 'arrangements')
    await flushPromises()
    const list = wrapper.findComponent({ name: 'StudyArrangements' })
    expect(list.props('active')).toBe(true)
    wrapper.findComponent({ name: 'ATabs' }).vm.$emit('update:activeKey', 'trend')
    await flushPromises()
    expect(list.props('active')).toBe(false)
    expect(wrapper.find('.arrangements-stub').exists()).toBe(true)
    wrapper.unmount()
  })
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(getBooks).mockResolvedValue([
      {
        id: 1,
        book_group: 1,
        product_code: 'book-1',
        title: '完整书名',
        short_title: '简称',
        author: null,
        product_type: 'imgbook',
        cover: null,
        sort_num: 1,
      },
    ])
    vi.mocked(getStudySessionsByDate).mockResolvedValue([])
    vi.mocked(getStudySessionPage).mockResolvedValue({ items: [], total: 0 })
    vi.mocked(getStudyStats).mockResolvedValue({
      periodType: 'week',
      rangeStart: '2026-08-05',
      rangeEnd: '2026-08-11',
      trend: [
        { date: '2026-08-05', duration: 0 },
        { date: '2026-08-06', duration: 61 },
      ],
      bookBreakdown: [],
      seriesBreakdown: [],
      recentSessions: [],
      page: 1,
      pageSize: 10,
      totalRecent: 0,
    })
  })

  it('aligns axis labels by percentage and only shows non-zero numeric bar values', async () => {
    const wrapper = mount(StudyStatsPage, {
      global: {
        stubs: {
          'a-spin': passthroughStub,
          'a-select': passthroughStub,
          'a-select-option': passthroughStub,
          'a-button': passthroughStub,
          'a-modal': passthroughStub,
        },
      },
    })

    await flushPromises()

    const axisLabels = wrapper.findAll('.y-label')
    expect(axisLabels.map((label) => label.text())).toEqual(['10', '0'])
    expect(axisLabels[0].attributes('style')).toContain('bottom: 100%')
    expect(axisLabels[1].attributes('style')).toContain('bottom: 0%')

    const barItems = wrapper.findAll('.trend-bar-item')
    expect(barItems[0].find('.trend-bar-value').exists()).toBe(false)
    expect(barItems[1].find('.trend-bar-value').text()).toBe('2')
    expect(wrapper.findAll('.trend-bar-value')).toHaveLength(1)
    expect(wrapper.text()).toContain('简称')
    expect(wrapper.text()).not.toContain('完整书名')
  })

  it('月视图和年视图均让趋势卡片占据整行', async () => {
    const wrapper = mount(StudyStatsPage, {
      global: {
        stubs: {
          'a-spin': passthroughStub,
          'a-select': passthroughStub,
          'a-select-option': passthroughStub,
          'a-button': passthroughStub,
          'a-modal': passthroughStub,
        },
      },
    })

    await flushPromises()
    const periodButtons = wrapper.findAll('.period-btn')

    await periodButtons[1].trigger('click')
    expect(wrapper.find('.stats-grid').classes()).toContain('is-wide-trend-view')

    await periodButtons[0].trigger('click')
    expect(wrapper.find('.stats-grid').classes()).not.toContain('is-wide-trend-view')

    await periodButtons[2].trigger('click')
    expect(wrapper.find('.stats-grid').classes()).toContain('is-wide-trend-view')
  })

  it('加载趋势详情时阻止重复请求并执行全局加载回调', async () => {
    let resolveSessions!: (sessions: never[]) => void
    vi.mocked(getStudySessionsByDate).mockReturnValue(
      new Promise((resolve) => {
        resolveSessions = resolve
      }),
    )
    const wrapper = mount(StudyStatsPage, {
      global: {
        stubs: {
          'a-spin': passthroughStub,
          'a-select': passthroughStub,
          'a-select-option': passthroughStub,
          'a-button': passthroughStub,
          'a-modal': passthroughStub,
        },
      },
    })
    await flushPromises()

    const activeBar = wrapper.findAll('.trend-bar-item')[1]
    void activeBar.trigger('click')
    void activeBar.trigger('click')
    await flushPromises()

    expect(getStudySessionsByDate).toHaveBeenCalledOnce()
    resolveSessions([])
    await flushPromises()
  })

  it('两个筛选框使用组件库原生下箭头', async () => {
    const wrapper = mount(StudyStatsPage, {
      global: {
        components: {
          ASelect: Select,
          ASelectOption: SelectOption,
        },
        stubs: {
          'a-spin': passthroughStub,
          'a-button': passthroughStub,
          'a-modal': passthroughStub,
        },
      },
    })

    await flushPromises()

    const arrows = wrapper.findAll('.stats-toolbar .ant-select-arrow .anticon-down')
    expect(arrows).toHaveLength(2)
    expect(wrapper.find('.filter-select-chevron').exists()).toBe(false)
  })

  it('学习记录明细提供修改和删除入口', async () => {
    vi.mocked(getStudySessionPage).mockResolvedValueOnce({
      items: [
        {
          id: 7,
          bookId: 1,
          bookGroup: 1,
          productCode: 'book-1',
          bookTitle: '简称',
          resourceId: 'RE_1',
          unitName: 'Describing character',
          entryResourceId: 'RE_1',
          entryUnitName: 'Unit 1',
          visitedUnits: [],
          startAt: '2026-08-10T10:00:00Z',
          endAt: '2026-08-10T10:01:00Z',
          duration: 60,
        },
      ],
      total: 1,
    })
    const wrapper = mount(StudyStatsPage, {
      global: {
        stubs: {
          'a-spin': passthroughStub,
          'a-select': passthroughStub,
          'a-select-option': passthroughStub,
          'a-button': { template: '<button><slot /></button>' },
          'a-modal': passthroughStub,
        },
      },
    })
    await flushPromises()

    wrapper.findComponent({ name: 'ATabs' }).vm.$emit('update:activeKey', 'records')
    await flushPromises()

    expect(wrapper.findAll('.recent-card .row-actions button')).toHaveLength(2)
    expect(wrapper.text()).toContain('Unit 1 Describing character')
    expect(wrapper.findAll('.recent-card .row-actions .operation-button')).toHaveLength(2)
  })
})
