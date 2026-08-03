import { ref, computed, watch } from 'vue'
import type { BookMetadata, TocNode } from '../types'

/**
 * 提供目录搜索、过滤和展开状态。
 * @param metadata - 当前图书元数据引用。
 */
export function useReaderTOC(metadata: { value: BookMetadata | null }) {
  const tocSearchText = ref('')
  const expandedKeys = ref<string[]>([])

  const searchNodes = (nodes: TocNode[], text: string): TocNode[] => {
    if (!text) return nodes
    return nodes
      .map((node) => ({ ...node }))
      .filter((node) => {
        if (node.children) {
          node.children = searchNodes(node.children, text)
        }
        return (
          node.title.toLowerCase().includes(text.toLowerCase()) ||
          (node.children && node.children.length > 0)
        )
      })
  }

  const filteredToc = computed(() => {
    if (!metadata.value) return []
    return searchNodes(metadata.value.toc, tocSearchText.value)
  })

  const getAllKeys = (nodes: TocNode[]): string[] => {
    let keys: string[] = []
    nodes.forEach((node) => {
      if (node.children?.length) {
        keys.push(node.key)
        keys.push(...getAllKeys(node.children))
      }
    })
    return keys
  }

  // 元数据或过滤结果变化后自动展开所有可见目录节点。
  watch(
    [() => metadata.value, () => filteredToc.value],
    ([meta, filtered]) => {
      if (meta || filtered) {
        expandedKeys.value = getAllKeys(filtered || meta?.toc || [])
      }
    },
    { immediate: true },
  )

  return {
    tocSearchText,
    expandedKeys,
    filteredToc,
  }
}
