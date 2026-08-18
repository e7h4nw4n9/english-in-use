import { flushPromises, mount } from '@vue/test-utils'
import { Select, SelectOption } from 'ant-design-vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import StudyStatsPage from '../StudyStatsPage.vue'
import { getBooks } from '../../../lib/api'
import { getStudySessionsByDate, getStudyStats } from '../../../lib/api/studyTimer'

vi.mock('../../../lib/api', () => ({
  getBooks: vi.fn(),
}))

vi.mock('../../../lib/api/studyTimer', () => ({
  getStudySessionsByDate: vi.fn(),
  getStudyStats: vi.fn(),
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

    const arrows = wrapper.findAll('.ant-select-arrow .anticon-down')
    expect(arrows).toHaveLength(2)
    expect(wrapper.find('.filter-select-chevron').exists()).toBe(false)
  })
})
