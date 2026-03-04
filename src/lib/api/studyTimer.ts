import { invoke } from '@tauri-apps/api/core'
import type {
  SaveStudySessionPayload,
  SaveStudySessionResponse,
  StudyStatsFilters,
  StudyStatsPeriodType,
  StudyStatsResponse,
} from '../../types'

export async function saveStudySession(
  payload: SaveStudySessionPayload,
): Promise<SaveStudySessionResponse> {
  return await invoke('save_study_session', { payload })
}

export async function getStudyStats(
  periodType: StudyStatsPeriodType,
  filters?: StudyStatsFilters,
  page: number = 1,
  pageSize: number = 20,
): Promise<StudyStatsResponse> {
  return await invoke('get_study_stats', { periodType, filters, page, pageSize })
}
