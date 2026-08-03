import { invoke } from '@tauri-apps/api/core'
import type {
  SaveStudySessionPayload,
  SaveStudySessionResponse,
  StudyStatsFilters,
  StudyStatsPeriodType,
  StudyStatsResponse,
  StudySessionListItem,
} from '../../types'
import { formatLocalDate } from '../datetime'

/**
 * 保存一次学习会话。
 * @param payload - 学习会话开始、结束、时长和单元信息。
 */
export async function saveStudySession(
  payload: SaveStudySessionPayload,
): Promise<SaveStudySessionResponse> {
  return await invoke('save_study_session', { payload })
}

/**
 * 查询分页学习统计。
 * @param periodType - 统计周期类型。
 * @param filters - 可选的图书和分组筛选条件。
 * @param page - 页码。
 * @param pageSize - 每页记录数。
 */
export async function getStudyStats(
  periodType: StudyStatsPeriodType,
  filters?: StudyStatsFilters,
  page: number = 1,
  pageSize: number = 20,
): Promise<StudyStatsResponse> {
  return await invoke('get_study_stats', {
    periodType,
    filters,
    page,
    pageSize,
    localDate: formatLocalDate(),
  })
}

/**
 * 查询指定日期的学习会话。
 * @param date - 本地日期。
 * @param filters - 可选的图书和分组筛选条件。
 */
export async function getStudySessionsByDate(
  date: string,
  filters?: StudyStatsFilters,
): Promise<StudySessionListItem[]> {
  return await invoke('get_study_sessions_by_date', { date, filters })
}
