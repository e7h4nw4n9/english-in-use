import { mount, flushPromises } from '@vue/test-utils'
import { defineComponent, ref } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { getStudySessionPage } from '@/lib/api/studyTimer'
import { recentStudyDateRange, useStudySessionList } from '../useStudySessionList'
import type { StudySessionPage, StudyStatsFilters } from '@/types'

vi.mock('@/lib/api/studyTimer', () => ({ getStudySessionPage: vi.fn() }))

/** 在组件生命周期中使用列表，返回控制筛选和可见性的测试入口。 */
function setup() {
  const filters = ref<StudyStatsFilters>({})
  const active = ref(true)
  let list!: ReturnType<typeof useStudySessionList>
  const wrapper = mount(
    defineComponent({
      setup() {
        list = useStudySessionList(filters, active)
        return () => null
      },
    }),
  )
  return {
    filters,
    active,
    wrapper,
    get list() {
      return list
    },
  }
}

beforeEach(() => {
  vi.mocked(getStudySessionPage).mockReset().mockResolvedValue({ items: [], total: 0 })
})
afterEach(() => {
  vi.useRealTimers()
})

describe('学习记录独立查询', () => {
  it('默认三十天按本地日历跨月计算', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 2, 1, 12))
    expect(recentStudyDateRange()).toEqual(['2026-01-31', '2026-03-01'])
  })

  it('筛选后回到第一页，隐藏时延迟加载且保留日期', async () => {
    vi.mocked(getStudySessionPage).mockResolvedValue({ items: [], total: 40 })
    const state = setup()
    await flushPromises()
    state.list.page.value = 3
    await flushPromises()
    state.list.dateRange.value = ['2026-08-01', '2026-08-31']
    await flushPromises()
    expect(getStudySessionPage).toHaveBeenLastCalledWith(
      expect.objectContaining({ page: 1, rangeStart: '2026-08-01', rangeEnd: '2026-08-31' }),
    )
    state.active.value = false
    await flushPromises()
    const count = vi.mocked(getStudySessionPage).mock.calls.length
    state.filters.value = { bookGroup: 2, bookId: 3 }
    await flushPromises()
    expect(getStudySessionPage).toHaveBeenCalledTimes(count)
    state.active.value = true
    await flushPromises()
    expect(getStudySessionPage).toHaveBeenLastCalledWith(
      expect.objectContaining({ bookGroup: 2, bookId: 3, rangeStart: '2026-08-01' }),
    )
    state.wrapper.unmount()
  })

  it('忽略旧筛选响应，末页删除后回退，并可重试失败查询', async () => {
    let resolve!: (value: StudySessionPage) => void
    vi.mocked(getStudySessionPage).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done
      }),
    )
    const state = setup()
    state.filters.value = { bookId: 2 }
    await flushPromises()
    resolve({ items: [], total: 99 })
    await flushPromises()
    expect(state.list.total.value).toBe(0)
    vi.mocked(getStudySessionPage).mockResolvedValue({ items: [], total: 11 })
    state.list.page.value = 2
    await flushPromises()
    vi.mocked(getStudySessionPage).mockResolvedValue({ items: [], total: 10 })
    await state.list.invalidate()
    await flushPromises()
    expect(state.list.page.value).toBe(1)
    expect(getStudySessionPage).toHaveBeenLastCalledWith(expect.objectContaining({ page: 1 }))
    vi.mocked(getStudySessionPage).mockRejectedValueOnce(new Error('offline'))
    await state.list.refresh()
    expect(state.list.error.value).toBe(true)
    await state.list.refresh()
    expect(state.list.error.value).toBe(false)
    state.wrapper.unmount()
  })
})
