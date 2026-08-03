import { invoke } from '@tauri-apps/api/core'
import type {
  CompleteStudyTaskResponse,
  StudyPlanActionResponse,
  StudyPlanStatusResponse,
  StudyPlanUpsertResponse,
  StudyTaskItem,
  StudyTaskSummaryResponse,
  StudyViewMode,
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
 * 完成学习任务并推进计划。
 * @param taskId - 学习任务标识。
 */
export async function completeStudyTask(taskId: number): Promise<CompleteStudyTaskResponse> {
  return await invoke('complete_study_task', { taskId })
}
