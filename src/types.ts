export type BookSourceType = 'Local' | 'CloudflareGateway'
export type DatabaseType = 'SQLite' | 'CloudflareGateway'

export interface LocalBookSource {
  type: 'Local'
  details: {
    path: string
  }
}

export interface CloudflareGatewayBookSource {
  type: 'CloudflareGateway'
  details: Record<string, never>
}

export type BookSource = LocalBookSource | CloudflareGatewayBookSource

export interface SQLiteDatabase {
  type: 'SQLite'
  details: {
    path: string
  }
}

export interface CloudflareGatewayDatabase {
  type: 'CloudflareGateway'
  details: Record<string, never>
}

export type DatabaseConnection = SQLiteDatabase | CloudflareGatewayDatabase

export interface CloudflareGatewayConfig {
  base_url: string
  access_token: string
}

export interface SystemConfig {
  language: string
  theme: 'system' | 'light' | 'dark'
  log_level: 'trace' | 'debug' | 'info' | 'warn' | 'error'
  enable_debug_tools: boolean
  enable_auto_check: boolean
  check_interval_mins: number
  auto_start_study_timer?: boolean
}

export interface AppConfig {
  system: SystemConfig
  book_source: BookSource | null
  database: DatabaseConnection | null
  cloudflare_gateway?: CloudflareGatewayConfig | null
  gateway_configuration_required?: boolean
}

export type ServiceStatusType =
  | { status: 'Connected' }
  | { status: 'Disconnected'; message: string }
  | { status: 'NotConfigured' }
  | { status: 'Testing' }

export interface ConnectionStatus {
  r2: ServiceStatusType
  database: ServiceStatusType
}

export interface AppInitProgress {
  message: string
  progress: number
}

export interface ExerciseDownloadProgressEvent {
  productCode: string
  resourceId?: string | null
  stage: 'deps' | 'resource'
  totalFiles: number
  completedFiles: number
  failedFiles: number
  percent: number
  done: boolean
}

export enum BookGroup {
  Vocabulary = 1,
  Grammar = 2,
}

export interface Book {
  id: number
  book_group: BookGroup
  product_code: string
  title: string
  author: string | null
  product_type: string
  cover: string | null
  sort_num: number
}

export interface TocNode {
  title: string
  key: string
  startPage?: string
  endPage?: string
  audioFiles?: OverlayAudio[]
  children?: TocNode[]
}

export interface ExerciseInfo {
  name: string
  resource_id: string
}

export interface OverlayAudio {
  path: string
  title?: string
}

export interface OverlayTargetPage {
  pagelabel: string
}

export interface LearningObject {
  course_id: string
  module_id: string
}

export interface OverlayItem {
  x: number
  y: number
  w: number
  h: number
  type: 'audio' | 'page' | 'exercise' | 'learning-object'
  audio?: OverlayAudio
  page?: OverlayTargetPage
  'learning-object'?: LearningObject
  exercise?: ExerciseInfo
}

export interface PageIndex {
  label: string
  image_path: string
  resource_id?: string
  exercises?: ExerciseInfo[]
  overlays?: OverlayItem[]
}

export interface BookMetadata {
  toc: TocNode[]
  exerciseToc?: TocNode[]
  pages: Record<string, PageIndex>
  pageLabels: string[]
  pageWidth: number
  pageHeight: number
}

export interface ReadingProgress {
  book_id: number
  resource_id: string | null
  page_label: string | null
  scale: number
  offset_x: number
  offset_y: number
  updated_at: string
}

export type StudyViewMode = 'day' | 'week' | 'month'

export interface StudyPlanUpsertResponse {
  planUnitId: number
  planStatus: 0 | 1 | 2
  nextReviewDate: string | null
  totalStages: number
  completedStages: number[]
}

export interface StudyPlanStatusResponse {
  inPlan: boolean
  planStatus: 0 | 1 | 2 | null
  planUnitId: number | null
  completedStages: number[]
  nextReviewDate: string | null
  overdueCount: number
}

export interface StudyPlanActionResponse {
  success: boolean
}

export interface StudyTaskSummaryDay {
  date: string
  total: number
  due: number
  overdue: number
  completed: number
}

export interface StudyTaskSummaryResponse {
  rangeStart: string
  rangeEnd: string
  viewMode: StudyViewMode
  days: StudyTaskSummaryDay[]
}

export interface StudyTaskItem {
  taskId: number
  planUnitId: number
  productCode: string
  resourceId: string
  unitName: string
  reviewStage: number
  scheduledDate: string
  isOverdue: boolean
  taskStatus: 0 | 1
  completedAt: string | null
}

export interface CompleteStudyTaskResponse {
  taskId: number
  taskStatus: 0 | 1
  planStatus: 0 | 1 | 2
  completedStages: number[]
}

export type StudyStatsPeriodType = 'week' | 'month' | 'year'

export interface StudySessionUnitRef {
  resourceId: string
  unitName: string
}

export interface SaveStudySessionPayload {
  productCode: string
  entryResourceId: string
  entryUnitName: string
  assignedResourceId: string
  assignedUnitName: string
  visitedUnits: StudySessionUnitRef[]
  startAt: string
  endAt: string
  duration: number
  localDate: string
  timezoneOffsetMinutes: number
}

export interface SaveStudySessionResponse {
  id: number
  success: boolean
}

export interface StudyStatsFilters {
  bookId?: number
  bookGroup?: number
}

export interface StudyStatsTrendItem {
  date: string
  duration: number
}

export interface StudyStatsBookBreakdownItem {
  bookId: number
  productCode: string
  bookTitle: string
  duration: number
}

export interface StudyStatsSeriesBreakdownItem {
  bookGroup: number
  seriesKey: string
  duration: number
}

export interface StudySessionListItem {
  id: number
  bookId: number
  bookGroup: number
  productCode: string
  bookTitle: string
  resourceId: string
  unitName: string
  entryResourceId: string
  entryUnitName: string
  visitedUnits: StudySessionUnitRef[]
  startAt: string
  endAt: string
  duration: number
}

export interface StudyStatsResponse {
  periodType: StudyStatsPeriodType
  rangeStart: string
  rangeEnd: string
  trend: StudyStatsTrendItem[]
  bookBreakdown: StudyStatsBookBreakdownItem[]
  seriesBreakdown: StudyStatsSeriesBreakdownItem[]
  recentSessions: StudySessionListItem[]
  page: number
  pageSize: number
  totalRecent: number
}
