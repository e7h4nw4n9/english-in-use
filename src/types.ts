export type BookSourceType = 'Local' | 'CloudflareR2'
export type DatabaseType = 'SQLite' | 'CloudflareD1'

export interface LocalBookSource {
  type: 'Local'
  details: {
    path: string
  }
}

export interface CloudflareR2BookSource {
  type: 'CloudflareR2'
  details: {
    account_id: string
    bucket_name: string
    access_key_id: string
    secret_access_key: string
    public_url?: string
  }
}

export type BookSource = LocalBookSource | CloudflareR2BookSource

export interface SQLiteDatabase {
  type: 'SQLite'
  details: {
    path: string
  }
}

export interface CloudflareD1Database {
  type: 'CloudflareD1'
  details: {
    account_id: string
    database_id: string
    api_token: string
  }
}

export type DatabaseConnection = SQLiteDatabase | CloudflareD1Database

export interface SystemConfig {
  language: string
  theme: 'system' | 'light' | 'dark'
  log_level: 'trace' | 'debug' | 'info' | 'warn' | 'error'
  enable_auto_check: boolean
  check_interval_mins: number
}

export interface AppConfig {
  system: SystemConfig
  book_source: BookSource | null
  database: DatabaseConnection | null
}

export type ServiceStatusType =
  | { status: 'Connected' }
  | { status: 'Disconnected'; message: string }
  | { status: 'NotConfigured' }
  | { status: 'Testing' }

export interface ConnectionStatus {
  r2: ServiceStatusType
  d1: ServiceStatusType
}

export interface AppInitProgress {
  message: string
  progress: number
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

export interface OverlayItem {
  x: number
  y: number
  w: number
  h: number
  type: 'audio' | 'page'
  audio?: OverlayAudio
  page?: OverlayTargetPage
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
