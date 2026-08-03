import type { Book, StudyTaskItem } from '../../types'

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

/** 将图书分组转换为任务列表使用的系列元数据。
 * @param book - 任务对应的图书。
 * @param t - 国际化文本解析函数。
 */
function getSeriesMeta(
  book: Book | undefined,
  t: (key: string) => string,
): { key: string; name: string; order: number } {
  if (!book) {
    return {
      key: 'unknown',
      name: t('studyPlan.unknownSeries'),
      order: 99,
    }
  }

  if (book.book_group === 1) {
    return {
      key: 'vocabulary',
      name: t('app.bookGroups.vocabulary'),
      order: 1,
    }
  }

  if (book.book_group === 2) {
    return {
      key: 'grammar',
      name: t('app.bookGroups.grammar'),
      order: 2,
    }
  }

  return {
    key: `group-${book.book_group}`,
    name: t('studyPlan.unknownSeries'),
    order: 90,
  }
}

/** 将任务按图书系列和级别顺序分组。
 * @param tasks - 需要分组的学习任务。
 * @param booksByCode - 按产品码索引的图书。
 * @param t - 国际化文本解析函数。
 */
export function groupTasksBySeries(
  tasks: StudyTaskItem[],
  booksByCode: Record<string, Book>,
  t: (key: string) => string,
): GroupedSeriesTasks[] {
  const seriesMap = new Map<
    string,
    {
      order: number
      seriesName: string
      booksMap: Map<string, { bookCode: string; bookTitle: string; tasks: StudyTaskItem[] }>
      total: number
    }
  >()

  for (const task of tasks) {
    const book = booksByCode[task.productCode]
    const { key, name, order } = getSeriesMeta(book, t)

    if (!seriesMap.has(key)) {
      seriesMap.set(key, {
        order,
        seriesName: name,
        booksMap: new Map(),
        total: 0,
      })
    }

    const seriesEntry = seriesMap.get(key)
    if (!seriesEntry) continue

    const bookCode = task.productCode || 'unknown'
    const bookTitle = book?.title || t('studyPlan.unknownBook')

    if (!seriesEntry.booksMap.has(bookCode)) {
      seriesEntry.booksMap.set(bookCode, {
        bookCode,
        bookTitle,
        tasks: [],
      })
    }

    const bookEntry = seriesEntry.booksMap.get(bookCode)
    if (!bookEntry) continue

    bookEntry.tasks.push(task)
    seriesEntry.total += 1
  }

  return Array.from(seriesMap.entries())
    .sort((a, b) => a[1].order - b[1].order)
    .map(([seriesKey, series]) => ({
      seriesKey,
      seriesName: series.seriesName,
      total: series.total,
      books: Array.from(series.booksMap.values())
        .map((book) => ({
          ...book,
          tasks: book.tasks.slice().sort((a, b) => {
            if (a.scheduledDate !== b.scheduledDate)
              return a.scheduledDate.localeCompare(b.scheduledDate)
            if (a.reviewStage !== b.reviewStage) return a.reviewStage - b.reviewStage
            return a.taskId - b.taskId
          }),
        }))
        .sort((a, b) => a.bookTitle.localeCompare(b.bookTitle)),
    }))
}
