import { describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { getStudyArrangements, getStudyArrangementReviews } from '../studyArrangements'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue([]) }))

describe('学习安排只读接口', () => {
  it('完整传递筛选分页和单元标识', async () => {
    const query = { bookId: 2, status: 'ended' as const, page: 3, pageSize: 10 }
    await getStudyArrangements(query)
    expect(invoke).toHaveBeenCalledWith('get_study_arrangements', { query })
    await getStudyArrangementReviews(18)
    expect(invoke).toHaveBeenCalledWith('get_study_arrangement_reviews', { planUnitId: 18 })
  })
})
