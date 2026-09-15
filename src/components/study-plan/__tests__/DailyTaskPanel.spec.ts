import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import DailyTaskPanel from '../DailyTaskPanel.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}))

describe('DailyTaskPanel', () => {
  it('显示书籍名称但不显示产品码', async () => {
    const wrapper = mount(DailyTaskPanel, {
      props: {
        groupedTasks: [
          {
            seriesKey: 'vocabulary',
            seriesName: '词汇',
            total: 1,
            books: [
              {
                bookCode: 'internal-product-code',
                bookTitle: '书籍简称',
                tasks: [
                  {
                    taskId: 1,
                    planUnitId: 1,
                    productCode: 'internal-product-code',
                    resourceId: 'RE_00012',
                    unitName: 'Describing character',
                    reviewStage: 1,
                    scheduledDate: '2026-08-13',
                    isOverdue: false,
                    taskStatus: 0,
                    completedAt: null,
                  },
                ],
              },
            ],
          },
        ],
        loading: false,
        showActions: false,
        completingTaskId: null,
      },
      global: {
        stubs: {
          'a-collapse': { template: '<div><slot /></div>' },
          'a-collapse-panel': { template: '<div><slot name="header" /><slot /></div>' },
          'a-button': { template: '<button><slot /></button>' },
          'a-tag': { template: '<span><slot /></span>' },
        },
      },
    })

    expect(wrapper.text()).toContain('书籍简称')
    expect(wrapper.get('.task-unit-name').text()).toBe('Unit 12 Describing character')
    expect(wrapper.text()).not.toContain('internal-product-code')
    expect(wrapper.find('.book-code-tag').exists()).toBe(false)
    expect(wrapper.text()).toContain('studyPlan.adjustPlan')
    expect(wrapper.findAll('.task-card-actions .operation-button')).toHaveLength(1)

    await wrapper.find('.task-card-actions button').trigger('click')
    expect(wrapper.emitted('adjustPlan')).toHaveLength(1)
  })
})
