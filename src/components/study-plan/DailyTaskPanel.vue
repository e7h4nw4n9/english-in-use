<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { theme } from 'ant-design-vue'
import {
  CheckOutlined,
  ExportOutlined,
  BookOutlined,
  FolderOutlined,
  RightOutlined,
} from '@ant-design/icons-vue'
import type { StudyTaskItem } from '../../types'
import type { GroupedSeriesTasks } from './taskGroups'

const props = defineProps<{
  groupedTasks: GroupedSeriesTasks[]
  loading: boolean
  showActions: boolean
  completingTaskId: number | null
}>()

const emit = defineEmits<{
  (e: 'jumpToStudy', task: StudyTaskItem): void
  (e: 'markTaskDone', task: StudyTaskItem): void
}>()

const { t } = useI18n()
const { useToken } = theme
const { token } = useToken()

const activeSeries = ref<string[]>([])
const activeBooks = ref<string[]>([])

// 自动展开所有系列和书籍
watch(
  () => props.groupedTasks,
  (newGroups) => {
    if (newGroups && newGroups.length > 0) {
      activeSeries.value = newGroups.map((s) => s.seriesKey)
      const allBookKeys: string[] = []
      newGroups.forEach((s) => {
        s.books.forEach((b) => {
          allBookKeys.push(`${s.seriesKey}-${b.bookCode}`)
        })
      })
      activeBooks.value = allBookKeys
    }
  },
  { immediate: true },
)

function onJumpToStudy(task: StudyTaskItem) {
  emit('jumpToStudy', task)
}

function onMarkTaskDone(task: StudyTaskItem) {
  emit('markTaskDone', task)
}
</script>

<template>
  <div v-if="props.loading" class="loading-state glass-block">
    <div class="loading-pulse"></div>
    <span>{{ t('app.loading') }}</span>
  </div>

  <div v-else-if="props.groupedTasks.length === 0" class="loading-state glass-block">
    <div class="empty-icon-wrapper">
      <CheckOutlined />
    </div>
    <p>{{ t('studyPlan.noTasksForDate') }}</p>
  </div>

  <div v-else class="daily-task-container">
    <a-collapse
      v-model:activeKey="activeSeries"
      ghost
      expand-icon-position="right"
      class="series-collapse"
    >
      <template #expandIcon="{ isActive }">
        <div class="custom-expand-icon" :class="{ 'is-active': isActive }">
          <RightOutlined />
        </div>
      </template>

      <a-collapse-panel v-for="series in props.groupedTasks" :key="series.seriesKey">
        <template #header>
          <div class="series-header">
            <div class="series-header-main">
              <div class="series-icon">
                <FolderOutlined />
              </div>
              <span class="series-name">{{ series.seriesName }}</span>
            </div>
            <div class="series-badge">
              <span class="badge-text">{{ series.total }}</span>
            </div>
          </div>
        </template>

        <div class="series-content">
          <a-collapse
            v-model:activeKey="activeBooks"
            ghost
            expand-icon-position="right"
            class="book-collapse"
          >
            <a-collapse-panel
              v-for="book in series.books"
              :key="`${series.seriesKey}-${book.bookCode}`"
            >
              <template #header>
                <div class="book-header">
                  <div class="book-header-info">
                    <BookOutlined class="book-header-icon" />
                    <span class="book-title">{{ book.bookTitle }}</span>
                  </div>
                  <span class="book-code-tag">{{ book.bookCode }}</span>
                </div>
              </template>

              <div class="task-list">
                <div
                  v-for="task in book.tasks"
                  :key="task.taskId"
                  class="task-glass-card"
                  :class="{
                    'is-done': task.taskStatus === 1,
                    'is-overdue': task.isOverdue && task.taskStatus !== 1,
                  }"
                >
                  <div class="task-card-body">
                    <div class="task-info">
                      <h4 class="task-unit-name">{{ task.unitName }}</h4>
                      <div class="task-meta-row">
                        <span class="meta-tag stage-tag">
                          {{ t('studyPlan.stage') }} {{ task.reviewStage }}
                        </span>
                        <span class="meta-date">{{ task.scheduledDate }}</span>
                        <a-tag v-if="task.taskStatus === 1" color="success" class="status-tag">
                          {{ t('studyPlan.done') }}
                        </a-tag>
                        <a-tag v-else-if="task.isOverdue" color="error" class="status-tag">
                          {{ t('studyPlan.overdue') }}
                        </a-tag>
                      </div>
                    </div>

                    <div v-if="props.showActions" class="task-card-actions">
                      <a-button
                        type="text"
                        class="action-btn study-btn"
                        @click="onJumpToStudy(task)"
                      >
                        <template #icon><ExportOutlined /></template>
                        {{ t('studyPlan.goStudy') }}
                      </a-button>
                      <a-button
                        type="primary"
                        size="small"
                        shape="round"
                        class="action-btn done-btn"
                        :disabled="task.taskStatus === 1"
                        :loading="props.completingTaskId === task.taskId"
                        @click="onMarkTaskDone(task)"
                      >
                        <template #icon><CheckOutlined /></template>
                        {{ t('studyPlan.complete') }}
                      </a-button>
                    </div>
                  </div>
                </div>
              </div>
            </a-collapse-panel>
          </a-collapse>
        </div>
      </a-collapse-panel>
    </a-collapse>
  </div>
</template>

<style scoped>
.daily-task-container {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.glass-block {
  background: color-mix(in srgb, v-bind('token.colorBgContainer') 40%, transparent);
  backdrop-filter: blur(20px);
  border: 1px solid color-mix(in srgb, #ffffff 20%, transparent);
  border-radius: 12px;
  padding: 16px;
}

/* Series Collapse Styling */
:deep(.series-collapse) {
  background: transparent;
}

:deep(.series-collapse > .ant-collapse-item) {
  margin-bottom: 4px;
  background: color-mix(in srgb, v-bind('token.colorBgContainer') 25%, transparent);
  backdrop-filter: blur(10px);
  border: 1px solid color-mix(in srgb, #ffffff 12%, transparent);
  border-radius: 12px !important;
  overflow: hidden;
  transition: all 0.3s ease;
}

:deep(.series-collapse > .ant-collapse-item:hover) {
  border-color: color-mix(in srgb, v-bind('token.colorPrimary') 25%, transparent);
}

:deep(.series-collapse > .ant-collapse-item > .ant-collapse-header) {
  padding: 6px 10px !important;
  align-items: center;
}

.series-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding-right: 4px;
}

.series-header-main {
  display: flex;
  align-items: center;
  gap: 8px;
}

.series-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  background: color-mix(in srgb, v-bind('token.colorPrimary') 12%, transparent);
  color: v-bind('token.colorPrimary');
  font-size: 12px;
}

.series-name {
  font-size: 13px;
  font-weight: 800;
  color: v-bind('token.colorTextHeading');
  letter-spacing: -0.01em;
}

.series-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 16px;
  padding: 0 5px;
  border-radius: 999px;
  background: v-bind('token.colorPrimary');
  color: #ffffff;
  font-size: 9px;
  font-weight: 800;
}

/* Book Collapse Styling */
.series-content {
  padding: 0 6px 6px;
}

:deep(.book-collapse > .ant-collapse-item) {
  margin-bottom: 2px;
  border: 1px solid color-mix(in srgb, #ffffff 8%, transparent);
  background: color-mix(in srgb, v-bind('token.colorFillAlter') 12%, transparent);
  border-radius: 10px !important;
  overflow: hidden;
}

:deep(.book-collapse > .ant-collapse-item > .ant-collapse-header) {
  padding: 4px 10px !important;
  font-size: 11px;
}

.book-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding-right: 4px;
}

.book-header-info {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.book-header-icon {
  color: v-bind('token.colorTextSecondary');
  opacity: 0.4;
  font-size: 12px;
}

.book-title {
  font-weight: 700;
  color: v-bind('token.colorTextSecondary');
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.book-code-tag {
  font-family: 'Plus Jakarta Sans', monospace;
  font-size: 8px;
  font-weight: 800;
  text-transform: uppercase;
  color: v-bind('token.colorTextTertiary');
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 25%, transparent);
  padding: 0px 4px;
  border-radius: 3px;
}

/* Task Card Styling */
.task-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 1px 0 4px;
}

.task-glass-card {
  position: relative;
  background: color-mix(in srgb, v-bind('token.colorBgElevated') 25%, transparent);
  border: 1px solid color-mix(in srgb, #ffffff 8%, transparent);
  border-radius: 8px;
  transition: all 0.2s cubic-bezier(0.23, 1, 0.32, 1);
  overflow: hidden;
}

.task-glass-card:hover {
  transform: translateX(3px);
  background: color-mix(in srgb, v-bind('token.colorBgElevated') 40%, transparent);
}

.task-card-body {
  padding: 6px 10px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.task-info {
  min-width: 0;
  flex: 1;
}

.task-unit-name {
  margin: 0 0 2px;
  font-size: 12px;
  font-weight: 700;
  color: v-bind('token.colorText');
  line-height: 1.2;
}

.task-meta-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
}

.meta-tag {
  font-size: 8px;
  font-weight: 700;
  padding: 0px 5px;
  border-radius: 3px;
}

.stage-tag {
  background: color-mix(in srgb, v-bind('token.colorInfo') 10%, transparent);
  color: v-bind('token.colorInfo');
}

.meta-date {
  font-size: 9px;
  color: v-bind('token.colorTextTertiary');
  font-weight: 600;
}

.status-tag {
  font-size: 8px;
  font-weight: 800;
  padding: 0 4px;
  height: 14px;
  line-height: 14px;
}

.task-card-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.action-btn {
  font-weight: 700;
  font-size: 10px;
  height: 24px;
  padding: 0 8px;
}

.study-btn {
  color: v-bind('token.colorPrimary');
  opacity: 0.6;
}

.study-btn:hover {
  opacity: 1;
  background: color-mix(in srgb, v-bind('token.colorPrimary') 10%, transparent);
}

.done-btn {
  box-shadow: 0 1px 4px color-mix(in srgb, v-bind('token.colorPrimary') 15%, transparent);
}

/* Status Variants */
.task-glass-card.is-done {
  opacity: 0.55;
  background: color-mix(in srgb, v-bind('token.colorSuccessBg') 10%, transparent);
}

.task-glass-card.is-overdue {
  border-left: 2px solid v-bind('token.colorError');
}

/* Custom Expand Icon */
.custom-expand-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  color: v-bind('token.colorTextTertiary');
  transition: all 0.3s ease;
  font-size: 8px;
}

.custom-expand-icon.is-active {
  transform: rotate(90deg);
  color: v-bind('token.colorPrimary');
  background: color-mix(in srgb, v-bind('token.colorPrimary') 10%, transparent);
}

/* Loading & Empty States */
.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  min-height: 160px;
  color: v-bind('token.colorTextTertiary');
  font-weight: 600;
  font-size: 12px;
}

.loading-pulse {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid color-mix(in srgb, v-bind('token.colorPrimary') 20%, transparent);
  border-top-color: v-bind('token.colorPrimary');
  animation: spin 1s linear infinite;
}

.empty-icon-wrapper {
  width: 32px;
  height: 32px;
  border-radius: 10px;
  background: color-mix(in srgb, v-bind('token.colorSuccess') 15%, transparent);
  color: v-bind('token.colorSuccess');
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  margin-bottom: 4px;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 768px) {
  .task-card-body {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .task-card-actions {
    width: auto;
    border-top: none;
    padding-top: 0;
  }
}
</style>
