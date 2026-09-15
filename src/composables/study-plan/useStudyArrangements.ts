import { computed, onBeforeUnmount, ref, watch, type Ref } from 'vue'
import type { StudyStatsFilters } from '@/types'
import {
  getStudyArrangements,
  getStudyArrangementReviews,
  type StudyArrangementQuery,
  type StudyArrangementList,
  type StudyArrangementReview,
} from '@/lib/api/studyArrangements'

/** 管理列表和明细缓存；active 表示计划页可见，filters 为下方公共筛选。 */
export function useStudyArrangements(
  active: Ref<boolean>,
  filters: Ref<StudyStatsFilters>,
  status: Ref<StudyArrangementQuery['status']>,
) {
  const page = ref(1)
  const result = ref<StudyArrangementList | null>(null)
  const loading = ref(false)
  const error = ref(false)
  const expanded = ref<number[]>([])
  const details = ref<Record<number, StudyArrangementReview[]>>({})
  const detailLoading = ref<Record<number, boolean>>({})
  const detailErrors = ref<Record<number, boolean>>({})
  let generation = 0
  const items = computed(() => result.value?.items ?? [])

  /** 刷新列表并使旧明细失效；过期响应不能覆盖新筛选结果。 */
  async function refresh() {
    const version = ++generation
    loading.value = true
    error.value = false
    expanded.value = []
    details.value = {}
    detailLoading.value = {}
    detailErrors.value = {}
    try {
      const response = await getStudyArrangements({
        ...filters.value,
        status: status.value,
        page: page.value,
        pageSize: 10,
      })
      if (version !== generation) return
      result.value = response
      page.value = response.page
    } catch {
      if (version === generation) {
        result.value = null
        error.value = true
      }
    } finally {
      if (version === generation) loading.value = false
    }
  }

  /** 加载一个单元的明细；id 为计划标识，同一轮列表中不重复请求。 */
  async function loadDetails(id: number) {
    if (details.value[id] || detailLoading.value[id]) return
    const version = generation
    detailLoading.value[id] = true
    detailErrors.value[id] = false
    try {
      const response = await getStudyArrangementReviews(id)
      if (version === generation) details.value[id] = response
    } catch {
      if (version === generation) detailErrors.value[id] = true
    } finally {
      if (version === generation) detailLoading.value[id] = false
    }
  }

  /** 切换指定计划行的展开状态；加载列表期间禁止展开旧数据。 */
  function toggle(id: number) {
    if (loading.value) return
    if (expanded.value.includes(id)) expanded.value = expanded.value.filter((value) => value !== id)
    else {
      expanded.value.push(id)
      void loadDetails(id)
    }
  }

  /** 切换分页；next 为目标页码。 */
  function changePage(next: number) {
    page.value = next
    void refresh()
  }
  watch([filters, status], () => {
    generation++
    page.value = 1
    if (active.value) void refresh()
  })
  watch(
    active,
    (visible) => {
      if (visible) void refresh()
    },
    { immediate: true },
  )
  onBeforeUnmount(() => {
    generation++
  })
  return {
    page,
    result,
    items,
    loading,
    error,
    expanded,
    details,
    detailLoading,
    detailErrors,
    refresh,
    loadDetails,
    toggle,
    changePage,
  }
}
