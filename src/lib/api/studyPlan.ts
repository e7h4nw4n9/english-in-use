import { invoke } from '@tauri-apps/api/core'
import type {
  CompleteStudyTaskResponse,
  StudyPlanActionResponse,
  StudyPlanStatusResponse,
  StudyPlanUpsertResponse,
  StudyTaskItem,
  StudyTaskSummaryResponse,
  StudyViewMode,
  ShiftStudyPlanResponse,
  StudyAssessmentPreview,
  BookStudyPlanStatusItem,
} from '../../types'
import { formatLocalDate } from '../datetime'

/**
 * 创建或更新学习单元计划。
 * @param productCode - 图书产品码。
 * @param resourceId - 学习单元资源标识。
 * @param unitName - 学习单元名称。
 */
export async function upsertStudyPlan(
  productCode: string,
  resourceId: string,
  unitName: string,
): Promise<StudyPlanUpsertResponse> {
  return await invoke('upsert_study_plan', {
    productCode,
    resourceId,
    unitName,
    localDate: formatLocalDate(),
  })
}

/**
 * 放弃学习单元计划。
 * @param productCode - 图书产品码。
 * @param resourceId - 学习单元资源标识。
 */
export async function abandonStudyPlan(
  productCode: string,
  resourceId: string,
): Promise<StudyPlanActionResponse> {
  return await invoke('abandon_study_plan', { productCode, resourceId })
}

/**
 * 查询学习单元计划状态。
 * @param productCode - 图书产品码。
 * @param resourceId - 学习单元资源标识。
 */
export async function getStudyPlanStatus(
  productCode: string,
  resourceId: string,
): Promise<StudyPlanStatusResponse> {
  return await invoke('get_study_plan_status', {
    productCode,
    resourceId,
    localDate: formatLocalDate(),
  })
}

/** 批量查询一本书内已有学习计划的单元状态。 */
export async function getBookStudyPlanStatuses(
  productCode: string,
): Promise<BookStudyPlanStatusItem[]> {
  return await invoke('get_book_study_plan_statuses', { productCode })
}

/**
 * 查询日期范围内的学习任务汇总。
 * @param rangeStart - 开始日期。
 * @param rangeEnd - 结束日期。
 * @param viewMode - 周或月视图模式。
 */
export async function getStudyTasksSummary(
  rangeStart: string,
  rangeEnd: string,
  viewMode: StudyViewMode,
): Promise<StudyTaskSummaryResponse> {
  return await invoke('get_study_tasks_summary', {
    rangeStart,
    rangeEnd,
    viewMode,
    localDate: formatLocalDate(),
  })
}

/**
 * 查询指定日期及逾期任务。
 * @param date - 目标本地日期。
 */
export async function getStudyTasksByDate(date: string): Promise<StudyTaskItem[]> {
  return await invoke('get_tasks_by_date', { date, localDate: formatLocalDate() })
}

/**
 * 整体平移第 1 阶段尚未完成的学习计划。
 * @param planUnitId - 学习计划标识。
 * @param offsetDays - 平移天数，负数表示提前，正数表示延期。
 */
export async function shiftStudyPlan(
  planUnitId: number,
  offsetDays: number,
): Promise<ShiftStudyPlanResponse> {
  return await invoke('shift_study_plan', {
    planUnitId,
    offsetDays,
    localDate: formatLocalDate(),
  })
}

/**
 * 完成学习任务并推进计划。
 * @param taskId - 学习任务标识。
 * @param rating - 第五次及以后任务的掌握评价。
 * @param finishPlan - 连续三次巩固后的结束选择。
 */
export async function completeStudyTask(
  taskId: number,
  rating?: 'forgotten' | 'hard' | 'good' | 'mastered',
  finishPlan?: boolean,
  expectedRevision?: string,
): Promise<CompleteStudyTaskResponse> {
  return await invoke('complete_study_task', {
    taskId,
    localDate: formatLocalDate(),
    rating,
    finishPlan,
    expectedRevision,
  })
}

/** 获取当前日期的评估预览；taskId 为未完成任务标识。 */
export async function getStudyAssessmentPreview(taskId: number): Promise<StudyAssessmentPreview> {
  return await invoke('get_study_assessment_preview', { taskId, localDate: formatLocalDate() })
}
