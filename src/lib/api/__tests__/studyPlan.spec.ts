import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import {
  getStudyPlanStatus,
  getStudyTasksByDate,
  getStudyTasksSummary,
  upsertStudyPlan,
} from '../studyPlan'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('study plan API local date', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 7, 10, 12, 0, 0))
    vi.clearAllMocks()
    vi.mocked(invoke).mockResolvedValue({})
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('passes the current local date to plan creation and status queries', async () => {
    await upsertStudyPlan('book', 'RE_1', 'Unit 1')
    await getStudyPlanStatus('book', 'RE_1')

    expect(invoke).toHaveBeenNthCalledWith(1, 'upsert_study_plan', {
      productCode: 'book',
      resourceId: 'RE_1',
      unitName: 'Unit 1',
      localDate: '2026-08-10',
    })
    expect(invoke).toHaveBeenNthCalledWith(2, 'get_study_plan_status', {
      productCode: 'book',
      resourceId: 'RE_1',
      localDate: '2026-08-10',
    })
  })

  it('passes the current local date to task list and summary queries', async () => {
    await getStudyTasksSummary('2026-08-10', '2026-08-16', 'week')
    await getStudyTasksByDate('2026-08-10')

    expect(invoke).toHaveBeenNthCalledWith(1, 'get_study_tasks_summary', {
      rangeStart: '2026-08-10',
      rangeEnd: '2026-08-16',
      viewMode: 'week',
      localDate: '2026-08-10',
    })
    expect(invoke).toHaveBeenNthCalledWith(2, 'get_tasks_by_date', {
      date: '2026-08-10',
      localDate: '2026-08-10',
    })
  })
})
