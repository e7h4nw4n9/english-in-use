<script setup lang="ts">
import { defineComponent } from 'vue'
import { storeToRefs } from 'pinia'
import { useReaderStore } from '../../stores/reader'
import type { BookMetadata } from '../../types'

defineProps<{
  metadata: BookMetadata | null
  sortedPageLabels: string[]
}>()

const readerStore = useReaderStore()
const { debugVisible, currentPageLabel, viewMode } = storeToRefs(readerStore)

const DebugTocNode = defineComponent({
  name: 'DebugTocNode',
  props: ['node', 'sortedLabels', 'currentIdx'],
  template: `
    <div class="mb-1">
      <div class="flex items-center gap-2" :class="{ 'bg-blue-100/50': isCurrent }">
        <span class="text-gray-400">[{{ range }}]</span>
        <span :class="{ 'text-blue-600 font-bold': node.audioFiles }">{{ node.title }}</span>
        <span v-if="node.audioFiles" class="text-red-500">({{ node.audioFiles.length }} audios)</span>
      </div>
      <div v-if="node.children" class="ml-4 border-l pl-2">
        <DebugTocNode v-for="child in node.children" :key="child.key" :node="child" :sorted-labels="sortedLabels" :current-idx="currentIdx" />
      </div>
    </div>
  `,
  computed: {
    range(): string {
      if (!this.node.startPage) return 'N/A'
      const s = this.sortedLabels.indexOf(this.node.startPage)
      const e = this.sortedLabels.indexOf(this.node.endPage)
      return `${s}-${e}`
    },
    isCurrent(): boolean {
      if (!this.node.startPage) return false
      const s = this.sortedLabels.indexOf(this.node.startPage)
      const e = this.sortedLabels.indexOf(this.node.endPage)
      return this.currentIdx >= s && this.currentIdx <= e
    },
  },
})
</script>

<template>
  <a-modal v-model:open="debugVisible" title="Reader Debug Info" :footer="null" width="600px">
    <div class="max-h-[70vh] space-y-4 overflow-y-auto p-4 font-mono text-[10px]">
      <div class="grid grid-cols-2 gap-2 rounded bg-gray-50 p-2 dark:bg-gray-800">
        <div class="flex justify-between border-b pb-1">
          <span>Current Page:</span> <span>{{ currentPageLabel }}</span>
        </div>
        <div class="flex justify-between border-b pb-1">
          <span>Page Index:</span> <span>{{ sortedPageLabels.indexOf(currentPageLabel) }}</span>
        </div>
        <div class="flex justify-between border-b pb-1">
          <span>View Mode:</span> <span>{{ viewMode }}</span>
        </div>
        <div class="flex justify-between border-b pb-1">
          <span>Labels Count:</span> <span>{{ sortedPageLabels.length }}</span>
        </div>
      </div>

      <div>
        <div class="mb-2 border-b font-bold text-blue-500">TOC Nodes & Ranges:</div>
        <div class="space-y-1">
          <template v-if="metadata">
            <div v-for="node in metadata.toc" :key="node.key" class="ml-0">
              <DebugTocNode
                :node="node"
                :sorted-labels="sortedPageLabels"
                :current-idx="sortedPageLabels.indexOf(currentPageLabel)"
              />
            </div>
          </template>
        </div>
      </div>

      <div>
        <div class="mb-2 border-b font-bold text-green-500">Page Labels Order (First 20):</div>
        <div class="flex flex-wrap gap-1">
          <span
            v-for="(l, i) in sortedPageLabels.slice(0, 20)"
            :key="i"
            class="rounded bg-gray-100 px-1 dark:bg-gray-700"
            :class="{ 'font-bold text-blue-500': l === currentPageLabel }"
          >
            {{ i }}:{{ l }}
          </span>
        </div>
      </div>
    </div>
  </a-modal>
</template>
