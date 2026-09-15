import { invoke } from '@tauri-apps/api/core'

export type StudyArrangementStatus = 'scheduled' | 'active' | 'ended'
export interface StudyArrangementQuery {
  bookId?: number
  bookGroup?: number
  status: StudyArrangementStatus | 'all'
  page: number
  pageSize: number
}
export interface StudyArrangementItem {
  planUnitId: number
  bookId: number
  bookTitle: string
  bookShortTitle: string | null
  totalDurationSeconds: number
  resourceId: string
  unitName: string
  status: StudyArrangementStatus
  completedCount: number
  nextReviewDate: string | null
}
export interface StudyArrangementList {
  items: StudyArrangementItem[]
  total: number
  page: number
  pageSize: number
}
export interface StudyArrangementReview {
  taskId: number
  reviewStage: number
  scheduledDate: string
  taskStatus: number
  completedAt: string | null
}

/** 查询单元列表及统计；query 为书籍、系列、状态和分页条件。 */
export function getStudyArrangements(query: StudyArrangementQuery): Promise<StudyArrangementList> {
  return invoke('get_study_arrangements', { query })
}

/** 查询计划现存的全部复习记录；planUnitId 为单元计划标识。 */
export function getStudyArrangementReviews(planUnitId: number): Promise<StudyArrangementReview[]> {
  return invoke('get_study_arrangement_reviews', { planUnitId })
}
