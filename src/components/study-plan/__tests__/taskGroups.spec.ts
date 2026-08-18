import { describe, expect, it } from 'vitest'
import type { Book, StudyTaskItem } from '../../../types'
import { groupTasksBySeries } from '../taskGroups'

function task(overrides: Partial<StudyTaskItem>): StudyTaskItem {
  return {
    taskId: 1,
    planUnitId: 1,
    productCode: 'book-a',
    resourceId: 'unit-1',
    unitName: 'Unit 1',
    reviewStage: 1,
    scheduledDate: '2026-03-02',
    isOverdue: false,
    taskStatus: 0,
    completedAt: null,
    ...overrides,
  }
}

describe('groupTasksBySeries', () => {
  it('按系列、图书和复习阶段稳定排序', () => {
    const books = {
      grammar: {
        product_code: 'grammar',
        title: 'Grammar',
        short_title: null,
        book_group: 2,
      } as Book,
      vocabulary: {
        product_code: 'vocabulary',
        title: 'Vocabulary',
        short_title: 'Vocab',
        book_group: 1,
      } as Book,
    }
    const translate = (key: string) => key
    const tasks = [
      task({ taskId: 3, productCode: 'grammar', reviewStage: 2 }),
      task({ taskId: 2, productCode: 'vocabulary', reviewStage: 2 }),
      task({ taskId: 1, productCode: 'vocabulary', reviewStage: 1 }),
    ]

    const groups = groupTasksBySeries(tasks, books, translate)

    expect(groups.map((group) => group.seriesKey)).toEqual(['vocabulary', 'grammar'])
    expect(groups[0].books[0].bookTitle).toBe('Vocab')
    expect(groups[0].books[0].tasks.map((item) => item.taskId)).toEqual([1, 2])
  })
})
