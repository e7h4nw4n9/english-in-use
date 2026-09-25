import { ref, watch, type Ref } from 'vue'
import { getBookStudyPlanStatuses } from '@/lib/api/studyPlan'
import type { BookMetadata, StudyPlanUnitStatus, TocNode } from '@/types'

interface UseReaderStudyPlanStatusesOptions {
  productCode: Ref<string | null | undefined>
  metadata: Ref<BookMetadata | null>
  refreshVersion: Ref<number>
}

/** 收集目录中可关联学习计划的单元资源标识。 */
function collectUnitResourceIds(metadata: BookMetadata): string[] {
  const resourceIds = new Set<string>()

  function visit(nodes: TocNode[]) {
    nodes.forEach((node) => {
      if (node.children?.length) {
        visit(node.children)
        return
      }
      const resourceId = node.startPage ? metadata.pages[node.startPage]?.resource_id : undefined
      if (resourceId) resourceIds.add(resourceId)
    })
  }

  visit(metadata.toc)
  return [...resourceIds]
}

/** 批量加载阅读器目录中的学习计划状态。 */
export function useReaderStudyPlanStatuses({
  productCode,
  metadata,
  refreshVersion,
}: UseReaderStudyPlanStatusesOptions) {
  const unitStatuses = ref<Record<string, StudyPlanUnitStatus>>({})
  let requestSequence = 0

  async function refresh() {
    const sequence = ++requestSequence
    const currentMetadata = metadata.value
    const code = productCode.value?.trim() || ''
    const resourceIds = currentMetadata ? collectUnitResourceIds(currentMetadata) : []
    const defaults = Object.fromEntries(
      resourceIds.map((resourceId) => [resourceId, 'unplanned' as const]),
    )
    unitStatuses.value = defaults
    if (!code || resourceIds.length === 0) return

    try {
      const items = await getBookStudyPlanStatuses(code)
      if (sequence !== requestSequence) return
      const nextStatuses: Record<string, StudyPlanUnitStatus> = { ...defaults }
      items.forEach((item) => {
        if (item.resourceId in nextStatuses) nextStatuses[item.resourceId] = item.status
      })
      unitStatuses.value = nextStatuses
    } catch {
      if (sequence === requestSequence) unitStatuses.value = defaults
    }
  }

  watch([productCode, metadata, refreshVersion], () => void refresh(), { immediate: true })

  return { unitStatuses, refresh }
}
