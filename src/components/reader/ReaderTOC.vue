<script setup lang="ts">
import { computed, watch, ref, nextTick } from 'vue'
import { storeToRefs } from 'pinia'
import { useReaderStore } from '../../stores/reader'
import { useReaderTOC } from '../../composables/useReaderTOC'
import type { BookMetadata, TocNode } from '../../types'
import { CloseOutlined, SearchOutlined, UnorderedListOutlined } from '@ant-design/icons-vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  metadata: BookMetadata | null
}>()

const { t } = useI18n()
const readerStore = useReaderStore()
const { isSidebarCollapsed, currentPageLabel, viewMode, spreadOffset } = storeToRefs(readerStore)

const { tocSearchText, expandedKeys, filteredToc } = useReaderTOC(computed(() => props.metadata))

const containerRef = ref()

// 自动滚动到当前活跃项
watch(isSidebarCollapsed, async (collapsed) => {
  if (!collapsed) {
    await nextTick()
    const activeItem = document.querySelector('.toc-item.is-active')
    if (activeItem) {
      activeItem.scrollIntoView({ behavior: 'smooth', block: 'center' })
    }
  }
})

function handleItemClick(node: TocNode) {
  const hasChildren = !!(node.children && node.children.length > 0)

  if (hasChildren) return

  if (node.startPage) {
    currentPageLabel.value = node.startPage

    if (viewMode.value === 'spread' && props.metadata) {
      const sortedLabels = props.metadata.pageLabels || Object.keys(props.metadata.pages).sort()
      const idx = sortedLabels.indexOf(node.startPage)
      if (idx !== -1) {
        const currentIsLeft = (idx - spreadOffset.value) % 2 === 0
        if (!currentIsLeft) {
          spreadOffset.value = spreadOffset.value === 0 ? 1 : 0
        }
      }
    }
  }
}
</script>

<template>
  <div class="reader-toc-wrapper pointer-events-none fixed bottom-0 left-0 top-8 z-[100] flex">
    <!-- 遮罩层 (仅在目录打开时显示) -->
    <Transition name="fade">
      <div
        v-if="!isSidebarCollapsed"
        class="toc-mask pointer-events-auto fixed inset-0 top-8 z-[101] bg-slate-900/20 backdrop-blur-[2px]"
        @click="isSidebarCollapsed = true"
      ></div>
    </Transition>

    <!-- 目录侧边栏 -->
    <aside
      class="toc-sidebar pointer-events-auto relative z-[102] flex h-full w-72 flex-col border-r border-slate-200 bg-white shadow-2xl dark:border-slate-800 dark:bg-slate-900"
    >
      <!-- 极简搜索栏 -->
      <header
        class="flex h-10 items-center border-b border-slate-200 bg-slate-50/30 dark:border-slate-800 dark:bg-slate-800/20"
      >
        <div class="group relative h-full flex-1">
          <a-input
            v-model:value="tocSearchText"
            :placeholder="t('reader.search')"
            allow-clear
            size="small"
            class="search-input !h-full !border-none !bg-transparent !px-3 !shadow-none"
          >
            <template #prefix>
              <SearchOutlined class="text-slate-400" />
            </template>
          </a-input>
        </div>
        <button
          class="flex h-full w-10 cursor-pointer items-center justify-center border-none bg-transparent text-slate-400 transition-colors hover:bg-slate-100 hover:text-red-500 dark:hover:bg-slate-800"
          @click="isSidebarCollapsed = true"
          :title="t('common.cancel')"
        >
          <CloseOutlined style="font-size: 14px" />
        </button>
      </header>

      <div class="custom-scrollbar flex-1 overflow-y-auto" ref="containerRef">
        <div class="py-1">
          <template v-for="item in filteredToc" :key="item.key">
            <a-collapse
              v-if="item.children && item.children.length > 0"
              v-model:activeKey="expandedKeys"
              ghost
              expand-icon-position="right"
              class="toc-parent-collapse"
            >
              <a-collapse-panel :key="item.key">
                <template #header>
                  <span
                    class="truncate text-[13px] font-bold text-slate-900 dark:text-slate-100"
                    :title="item.title"
                  >
                    {{ item.title }}
                  </span>
                </template>

                <div
                  class="toc-children-container ml-3 border-l border-slate-100 dark:border-slate-800"
                >
                  <div
                    v-for="child in item.children"
                    :key="child.key"
                    class="toc-item group relative flex cursor-pointer items-center justify-between px-3 py-2 pl-4 transition-colors"
                    :class="[
                      currentPageLabel === child.startPage
                        ? 'is-active bg-slate-100 dark:bg-slate-800'
                        : 'hover:bg-slate-50 dark:hover:bg-slate-800/50',
                    ]"
                    @click="handleItemClick(child)"
                  >
                    <div class="flex flex-1 items-center gap-2 overflow-hidden">
                      <span
                        class="truncate text-[13px]"
                        :class="[
                          currentPageLabel === child.startPage
                            ? 'font-bold text-blue-600 dark:text-blue-400'
                            : 'font-medium text-slate-600 dark:text-slate-400',
                        ]"
                      >
                        {{ child.title }}
                      </span>
                    </div>
                    <div class="ml-2 flex shrink-0 items-center gap-2">
                      <span
                        v-if="child.startPage"
                        class="font-mono text-[10px] text-slate-400"
                        :class="{
                          'font-bold text-blue-600 dark:text-blue-400':
                            currentPageLabel === child.startPage,
                        }"
                      >
                        {{ child.startPage }}
                      </span>
                    </div>
                  </div>
                </div>
              </a-collapse-panel>
            </a-collapse>

            <div
              v-else
              class="toc-item group relative flex cursor-pointer items-center justify-between px-3 py-2 transition-colors"
              :class="[
                currentPageLabel === item.startPage
                  ? 'is-active bg-slate-100 dark:bg-slate-800'
                  : 'hover:bg-slate-50 dark:hover:bg-slate-800/50',
              ]"
              @click="handleItemClick(item)"
            >
              <div class="flex flex-1 items-center gap-2 overflow-hidden">
                <span
                  class="truncate text-[13px]"
                  :class="[
                    currentPageLabel === item.startPage
                      ? 'font-bold text-blue-600 dark:text-blue-400'
                      : 'font-semibold text-slate-900 dark:text-slate-100',
                  ]"
                >
                  {{ item.title }}
                </span>
              </div>
              <div class="ml-2 flex shrink-0 items-center gap-2">
                <span
                  v-if="item.startPage"
                  class="font-mono text-[10px] text-slate-400"
                  :class="{
                    'font-bold text-blue-600 dark:text-blue-400':
                      currentPageLabel === item.startPage,
                  }"
                >
                  {{ item.startPage }}
                </span>
              </div>
            </div>
          </template>
        </div>
        <div
          v-if="filteredToc.length === 0"
          class="flex flex-col items-center justify-center px-4 py-20 text-slate-400"
        >
          <p class="text-xs">{{ tocSearchText ? t('reader.noResults') : t('reader.noContent') }}</p>
        </div>
      </div>

      <footer
        class="border-t border-slate-200 bg-slate-50/50 px-3 py-2 dark:border-slate-800 dark:bg-slate-800/20"
      >
        <div class="mb-1 flex items-center justify-between font-mono text-[10px] text-slate-500">
          <span>{{ t('reader.progress') }}</span>
          <span
            >{{
              Math.round(
                ((metadata?.pageLabels?.indexOf(currentPageLabel) || 0) /
                  (metadata?.pageLabels?.length || 1)) *
                  100,
              )
            }}%</span
          >
        </div>
        <div
          class="h-1 w-full overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700"
          v-if="metadata"
        >
          <div
            class="h-full bg-blue-600 transition-all duration-300"
            :style="{
              width: `${((metadata.pageLabels?.indexOf(currentPageLabel) || 0) / (metadata.pageLabels?.length || 1)) * 100}%`,
            }"
          ></div>
        </div>
      </footer>
    </aside>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 10px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: #334155;
}

:deep(.toc-parent-collapse .ant-collapse-header) {
  padding: 8px 12px !important;
  align-items: center !important;
}

:deep(.toc-parent-collapse .ant-collapse-content-box) {
  padding: 0 !important;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.4s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
