import { ref, computed, watch } from 'vue'
import type { BookMetadata, TocNode } from '../types'

export function useReaderTOC(metadata: { value: BookMetadata | null }) {
  const tocSearchText = ref('')
  const expandedKeys = ref<string[]>([])

  const filteredToc = computed(() => {
    if (!metadata.value) return []
    if (!tocSearchText.value) return metadata.value.toc

    const search = (nodes: TocNode[]): TocNode[] => {
      return nodes
        .map((node) => ({ ...node }))
        .filter((node) => {
          if (node.children) {
            node.children = search(node.children)
          }
          return (
            node.title.toLowerCase().includes(tocSearchText.value.toLowerCase()) ||
            (node.children && node.children.length > 0)
          )
        })
    }

    return search(metadata.value.toc)
  })

  // Auto-expand all nodes when metadata or filter changes
  watch(
    [() => metadata.value, () => filteredToc.value],
    ([meta, filtered]) => {
      if (meta || filtered) {
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
