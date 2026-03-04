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

export async function upsertStudyPlan(
  productCode: string,
  resourceId: string,
  unitName: string,
): Promise<StudyPlanUpsertResponse> {
  return await invoke('upsert_study_plan', { productCode, resourceId, unitName })
}

export async function abandonStudyPlan(
  productCode: string,
  resourceId: string,
): Promise<StudyPlanActionResponse> {
  return await invoke('abandon_study_plan', { productCode, resourceId })
}

export async function getStudyPlanStatus(
  productCode: string,
  resourceId: string,
): Promise<StudyPlanStatusResponse> {
  return await invoke('get_study_plan_status', { productCode, resourceId })
}

export async function getStudyTasksSummary(
  rangeStart: string,
  rangeEnd: string,
  viewMode: StudyViewMode,
): Promise<StudyTaskSummaryResponse> {
  return await invoke('get_study_tasks_summary', { rangeStart, rangeEnd, viewMode })
}

export async function getStudyTasksByDate(date: string): Promise<StudyTaskItem[]> {
  return await invoke('get_tasks_by_date', { date })
}

export async function completeStudyTask(taskId: number): Promise<CompleteStudyTaskResponse> {
  return await invoke('complete_study_task', { taskId })
}
