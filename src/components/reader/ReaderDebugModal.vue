<script setup lang="ts">
import { defineComponent, computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useReaderStore } from '../../stores/reader'
import type { BookMetadata } from '../../types'

const props = defineProps<{
  metadata: BookMetadata | null
  sortedPageLabels: string[]
}>()

const readerStore = useReaderStore()
const { debugVisible, currentPageLabel, viewMode } = storeToRefs(readerStore)

const currentPageExercises = computed(() => {
  if (!props.metadata) return []
  const labels: string[] = [currentPageLabel.value]

  if (viewMode.value === 'spread') {
    const idx = props.sortedPageLabels.indexOf(currentPageLabel.value)
    if (idx !== -1 && idx + 1 < props.sortedPageLabels.length) {
      labels.push(props.sortedPageLabels[idx + 1])
    }
  }

  const exercises: any[] = []
  labels.forEach((label) => {
    const page = props.metadata?.pages[label]
    if (page?.exercises) {
      page.exercises.forEach((ex) => {
        exercises.push({ ...ex, pageLabel: label })
      })
    }
  })
  return exercises
})

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
  <a-modal
    v-model:open="debugVisible"
    title="Reader Debug Info"
    :footer="null"
    width="600px"
    destroy-on-close
  >
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

      <!-- Current Exercises Section -->
      <div>
        <div class="mb-2 border-b font-bold text-orange-500">Current Exercises:</div>
        <div v-if="currentPageExercises.length > 0" class="space-y-1">
          <div
            v-for="(ex, i) in currentPageExercises"
            :key="i"
            class="flex items-center gap-2 rounded bg-orange-50/50 p-1 dark:bg-orange-950/20"
          >
            <span class="font-bold text-orange-600 dark:text-orange-400">[{{ ex.pageLabel }}]</span>
            <span class="text-slate-700 dark:text-slate-300">{{ ex.name }}</span>
            <span class="ml-auto text-[8px] opacity-40">ID: {{ ex.resource_id }}</span>
          </div>
        </div>
        <div v-else class="py-2 italic text-slate-400">No exercises found for current page.</div>
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
