import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import {
  getStudyPlanStatus,
  getStudyTasksByDate,
  getStudyTasksSummary,
  shiftStudyPlan,
  upsertStudyPlan,
  getStudyAssessmentPreview,
  completeStudyTask,
  getBookStudyPlanStatuses,
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

  it('评估预览和提交传递本地日期与预览版本', async () => {
    await getStudyAssessmentPreview(5)
    await completeStudyTask(5, 'good', undefined, 'revision')
    expect(invoke).toHaveBeenNthCalledWith(1, 'get_study_assessment_preview', {
      taskId: 5,
      localDate: '2026-08-10',
    })
    expect(invoke).toHaveBeenNthCalledWith(2, 'complete_study_task', {
      taskId: 5,
      localDate: '2026-08-10',
      rating: 'good',
      finishPlan: undefined,
      expectedRevision: 'revision',
    })
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

  it('queries all study plan statuses for one book in a single request', async () => {
    await getBookStudyPlanStatuses('book')

    expect(invoke).toHaveBeenCalledWith('get_book_study_plan_statuses', {
      productCode: 'book',
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

  it('passes the current local date to plan shift', async () => {
    await shiftStudyPlan(42, -2)

    expect(invoke).toHaveBeenCalledWith('shift_study_plan', {
      planUnitId: 42,
      offsetDays: -2,
      localDate: '2026-08-10',
    })
  })
})
