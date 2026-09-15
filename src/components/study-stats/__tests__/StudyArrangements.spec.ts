import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createI18n } from 'vue-i18n'
import { Alert, Button, Tag } from 'ant-design-vue'
import zh from '@/locales/zh.json'
import StudyArrangements from '../StudyArrangements.vue'
import {
  getStudyArrangements,
  getStudyArrangementReviews,
  type StudyArrangementList,
} from '@/lib/api/studyArrangements'

vi.mock('@/lib/api/studyArrangements', () => ({
  getStudyArrangements: vi.fn(),
  getStudyArrangementReviews: vi.fn(),
}))

/** 构造单元列表响应。 */
function fixture(): StudyArrangementList {
  return {
    items: [
      {
        planUnitId: 1,
        bookId: 1,
        bookTitle: 'Vocabulary',
        bookShortTitle: ' Vocab ',
        totalDurationSeconds: 90061,
        resourceId: 'RE_2',
        unitName: 'Character',
        status: 'active',
        completedCount: 1,
        nextReviewDate: '2026-10-01',
      },
    ],
    total: 21,
    page: 1,
    pageSize: 10,
  }
}
/** 仅注册入口已有组件，避免全量注册掩盖页面缺失的局部导入。 */
function render() {
  return mount(StudyArrangements, {
    props: { active: true, filters: {}, status: 'all' },
    global: {
      plugins: [Alert, Button, Tag, createI18n({ legacy: false, locale: 'zh', messages: { zh } })],
      config: {
        warnHandler: (message) => {
          if (message.includes('Failed to resolve component')) throw new Error(message)
        },
      },
    },
  })
}
beforeEach(() => {
  vi.mocked(getStudyArrangements).mockReset().mockResolvedValue(fixture())
  vi.mocked(getStudyArrangementReviews)
    .mockReset()
    .mockResolvedValue([
      { taskId: 1, reviewStage: 1, scheduledDate: '2000-01-01', taskStatus: 0, completedAt: null },
    ])
})

describe('学习安排', () => {
  it('简称为空时回退全称，零时长显示完整时分秒', async () => {
    const response = fixture()
    response.items[0]!.bookShortTitle = '  '
    response.items[0]!.totalDurationSeconds = 0
    vi.mocked(getStudyArrangements).mockResolvedValue(response)
    const wrapper = render()
    await flushPromises()
    expect(wrapper.text()).toContain('Vocabulary')
    expect(wrapper.text()).toContain('00:00:00')
    wrapper.unmount()
  })
  it('按单元显示，整行展开并缓存明细，刷新后失效', async () => {
    const wrapper = render()
    await flushPromises()
    expect(wrapper.text()).toContain('Unit 2 Character')
    expect(wrapper.text()).toContain('Vocab')
    expect(wrapper.text()).toContain('25:01:01')
    expect(wrapper.find('table').exists()).toBe(true)
    expect(wrapper.find('.arrangement-summary').exists()).toBe(false)
    expect(wrapper.find('.arrangement-toolbar').exists()).toBe(false)
    // expect(wrapper.get('.arrangement-row td:first-child .expand-button').exists()).toBe(true)
    expect(wrapper.findAll('.arrangement-row .expand-button')).toHaveLength(1)
    const columns = wrapper.findComponent({ name: 'ATable' }).props('columns')
    expect(columns[0].key).toBe('expand')
    expect(columns.find((column: { key: string }) => column.key === 'book').width).toBeUndefined()
    expect(columns.find((column: { key: string }) => column.key === 'unit').width).toBeUndefined()
    expect(wrapper.find('.ant-pagination').exists()).toBe(true)
    await wrapper.get('.arrangement-row').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('第 1 次')
    expect(wrapper.text()).toContain('已逾期')
    await wrapper.get('.expand-button').trigger('click')
    await wrapper.get('.expand-button').trigger('click')
    await flushPromises()
    expect(getStudyArrangementReviews).toHaveBeenCalledTimes(1)
    await (wrapper.vm as unknown as { refresh: () => Promise<void> }).refresh()
    await flushPromises()
    await wrapper.get('.expand-button').trigger('click')
    await flushPromises()
    expect(getStudyArrangementReviews).toHaveBeenCalledTimes(2)
    wrapper.unmount()
  })

  it('筛选重置分页，再次进入刷新并保留状态条件', async () => {
    const wrapper = render()
    await flushPromises()
    wrapper.findComponent({ name: 'APagination' }).vm.$emit('change', 2)
    await flushPromises()
    expect(getStudyArrangements).toHaveBeenLastCalledWith(expect.objectContaining({ page: 2 }))
    await wrapper.setProps({ status: 'ended' })
    await flushPromises()
    expect(getStudyArrangements).toHaveBeenLastCalledWith({
      status: 'ended',
      page: 1,
      pageSize: 10,
    })
    await wrapper.setProps({ active: false })
    const before = vi.mocked(getStudyArrangements).mock.calls.length
    await wrapper.setProps({ active: true })
    await flushPromises()
    expect(getStudyArrangements).toHaveBeenCalledTimes(before + 1)
    expect(getStudyArrangements).toHaveBeenLastCalledWith(
      expect.objectContaining({ status: 'ended' }),
    )
    wrapper.unmount()
  })

  it('空结果显示提示且不显示分页', async () => {
    vi.mocked(getStudyArrangements).mockResolvedValue({ ...fixture(), items: [], total: 0 })
    const wrapper = render()
    await flushPromises()
    expect(wrapper.text()).toContain('暂无符合条件的学习安排')
    expect(wrapper.find('.ant-pagination').exists()).toBe(false)
    wrapper.unmount()
  })

  it('列表和明细失败可分别重试', async () => {
    vi.mocked(getStudyArrangements).mockRejectedValueOnce(new Error('offline'))
    const wrapper = render()
    await flushPromises()
    expect(wrapper.text()).toContain('加载失败')
    await wrapper.get('.ant-alert button').trigger('click')
    await flushPromises()
    vi.mocked(getStudyArrangementReviews).mockRejectedValueOnce(new Error('offline'))
    await wrapper.get('.expand-button').trigger('click')
    await flushPromises()
    expect(wrapper.get('.review-details').text()).toContain('加载失败')
    await wrapper.get('.review-details button').trigger('click')
    await flushPromises()
    expect(wrapper.get('.review-details').text()).toContain('第 1 次')
    wrapper.unmount()
  })

  it('刷新后忽略上一轮明细的迟到响应', async () => {
    let resolve!: (value: []) => void
    vi.mocked(getStudyArrangementReviews).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done
      }),
    )
    const wrapper = render()
    await flushPromises()
    await wrapper.get('.expand-button').trigger('click')
    await (wrapper.vm as unknown as { refresh: () => Promise<void> }).refresh()
    await flushPromises()
    resolve([])
    await flushPromises()
    await wrapper.get('.expand-button').trigger('click')
    await flushPromises()
    expect(getStudyArrangementReviews).toHaveBeenCalledTimes(2)
    expect(wrapper.text()).toContain('第 1 次')
    wrapper.unmount()
  })
})
