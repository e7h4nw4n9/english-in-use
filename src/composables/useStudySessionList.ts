import { onBeforeUnmount, ref, watch, type Ref } from 'vue'
import type { StudySessionListItem, StudyStatsFilters } from '@/types'
import { getStudySessionPage } from '@/lib/api/studyTimer'
import { formatLocalDate } from '@/lib/datetime'

/** 返回包含今天的最近三十个本地日历日。 */
export function recentStudyDateRange(): [string, string] {
  const today = new Date()
  const start = new Date(today)
  start.setDate(start.getDate() - 29)
  return [formatLocalDate(start), formatLocalDate(today)]
}

/** 管理独立的记录列表；filters 为下方公共筛选，active 表示记录页是否可见。 */
export function useStudySessionList(filters: Ref<StudyStatsFilters>, active: Ref<boolean>) {
  const dateRange = ref<[string, string]>(recentStudyDateRange())
  const page = ref(1)
  const pageSize = 10
  const items = ref<StudySessionListItem[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref(false)
  let version = 0
  let dirty = true

  /** 查询当前条件；删除末页记录时回退至最后一个有效页。 */
  async function refresh() {
    const current = ++version
    loading.value = true
    error.value = false
    try {
      const response = await getStudySessionPage({
        ...filters.value,
        rangeStart: dateRange.value[0],
        rangeEnd: dateRange.value[1],
        page: page.value,
        pageSize,
      })
      if (current !== version) return
      total.value = response.total
      const lastPage = Math.max(1, Math.ceil(response.total / pageSize))
      if (page.value > lastPage) {
        page.value = lastPage
        return
      }
      items.value = response.items
      dirty = false
    } catch {
      if (current !== version) return
      items.value = []
      total.value = 0
      error.value = true
    } finally {
      if (current === version) loading.value = false
    }
  }

  /** 标记修改后的数据失效；隐藏时延迟查询。 */
  async function invalidate() {
    version++
    dirty = true
    if (active.value) await refresh()
  }

  watch([filters, dateRange], () => {
    version++
    dirty = true
    if (page.value !== 1) page.value = 1
    else if (active.value) void refresh()
  })
  watch(page, () => {
    dirty = true
    if (active.value) void refresh()
  })
  watch(
    active,
    (visible) => {
      if (visible && dirty) void refresh()
    },
    { immediate: true },
  )
  onBeforeUnmount(() => {
    version++
  })
  return { dateRange, page, pageSize, items, total, loading, error, refresh, invalidate }
}
