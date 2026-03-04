import type { StudyTaskItem } from '../../types'

export interface GroupedBookTasks {
  bookCode: string
  bookTitle: string
  tasks: StudyTaskItem[]
}

export interface GroupedSeriesTasks {
  seriesKey: string
  seriesName: string
  books: GroupedBookTasks[]
  total: number
}
