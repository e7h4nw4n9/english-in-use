<script setup lang="ts">
import { computed, toRef } from 'vue'
import type { TableColumnsType } from 'ant-design-vue'
import { theme, Grid, Table as ATable, Pagination as APagination } from 'ant-design-vue'
import { RightOutlined } from '@ant-design/icons-vue'
import { useI18n } from 'vue-i18n'
import type { StudyStatsFilters } from '@/types'
import type { StudyArrangementItem, StudyArrangementQuery } from '@/lib/api/studyArrangements'
import { useStudyArrangements } from '@/composables/study-plan/useStudyArrangements'
import { formatUnitTitle } from '@/lib/unitTitle'
import StudyArrangementReviews from './StudyArrangementReviews.vue'

const props = defineProps<{
  active: boolean
  filters: StudyStatsFilters
  status: StudyArrangementQuery['status']
}>()
const emit = defineEmits<{
  (event: 'update:status', value: StudyArrangementQuery['status']): void
}>()
const { t } = useI18n()
const { token } = theme.useToken()
const {
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
} = useStudyArrangements(
  toRef(props, 'active'),
  toRef(props, 'filters'),
  computed({
    get: () => props.status,
    set: (value) => emit('update:status', value),
  }),
)

defineExpose({ refresh })
const colors = { scheduled: 'default', active: 'blue', ended: 'green' }
type ArrangementRow = StudyArrangementItem & { isDetail?: boolean; rowKey: string }
const screens = Grid.useBreakpoint()
// 当前 Ant Design 预发布版本的内置展开列会在响应式更新时错误调用 renderSlot。
// 使用受控明细行，继续复用表格而不修改依赖或业务记录。
const tableRows = computed<ArrangementRow[]>(() =>
  items.value.flatMap((item) => {
    const row = { ...item, rowKey: String(item.planUnitId) }
    return expanded.value.includes(item.planUnitId)
      ? [row, { ...row, isDetail: true, rowKey: `${item.planUnitId}-detail` }]
      : [row]
  }),
)
/** 明细行仅保留跨列的单元格；record 为展示行。 */
function hiddenDetailCell(record: ArrangementRow) {
  return record.isDetail ? { colSpan: 0 } : {}
}
const columns = computed<TableColumnsType<ArrangementRow>>(() => [
  {
    key: 'expand',
    width: 56,
    customCell: (record: ArrangementRow) =>
      record.isDetail ? { colSpan: screens.value.md ? 7 : 2 } : {},
  },
  {
    key: 'book',
    title: t('studyArrangements.book'),
    responsive: ['md'],
    customCell: hiddenDetailCell,
  },
  {
    key: 'unit',
    title: t('studyArrangements.unit'),
    customCell: hiddenDetailCell,
  },
  {
    key: 'duration',
    title: t('studyArrangements.duration'),
    width: 120,
    responsive: ['md'],
    customCell: hiddenDetailCell,
  },
  {
    key: 'status',
    title: t('studyArrangements.status'),
    width: 110,
    responsive: ['md'],
    customCell: hiddenDetailCell,
  },
  {
    key: 'completed',
    title: t('studyArrangements.completedCount'),
    width: 110,
    responsive: ['md'],
    customCell: hiddenDetailCell,
  },
  {
    key: 'next',
    title: t('studyArrangements.next'),
    width: 140,
    responsive: ['md'],
    customCell: hiddenDetailCell,
  },
])
/** 整行点击展开，按钮自身通过 stop 避免触发两次；record 为单元记录。 */
function rowProps(record: ArrangementRow) {
  return record.isDetail
    ? { class: 'arrangement-detail-row' }
    : { onClick: () => toggle(record.planUnitId), class: 'arrangement-row' }
}

/** 列表和筛选器共用简称回退规则；shortTitle 为简称，title 为完整书名。 */
function bookLabel(shortTitle: string | null, title: string): string {
  return shortTitle?.trim() || title
}

/** 将单元累计秒数显示为时分秒，小时不按天取模。 */
function durationLabel(seconds: number): string {
  const value = Math.max(0, Math.floor(seconds))
  return [Math.floor(value / 3600), Math.floor(value / 60) % 60, value % 60]
    .map((part) => String(part).padStart(2, '0'))
    .join(':')
}
</script>

<template>
  <div class="arrangements">
    <a-alert v-if="error" type="error" :message="t('studyArrangements.loadError')" show-icon>
      <template #action
        ><a-button @click="refresh">{{ t('studyArrangements.retry') }}</a-button></template
      >
    </a-alert>
    <a-table
      v-else
      class="arrangement-table"
      table-layout="fixed"
      :columns="columns"
      :data-source="tableRows"
      row-key="rowKey"
      :loading="loading"
      :pagination="false"
      :custom-row="rowProps"
      :locale="{ emptyText: t('studyArrangements.empty') }"
    >
      <template #bodyCell="{ column, record }">
        <StudyArrangementReviews
          v-if="record.isDetail && column.key === 'expand'"
          :reviews="details[record.planUnitId]"
          :loading="detailLoading[record.planUnitId]"
          :error="detailErrors[record.planUnitId]"
          @retry="loadDetails(record.planUnitId)"
        />
        <template v-else-if="column.key === 'expand'">
          <button
            class="expand-button"
            :aria-label="
              t(
                expanded.includes(record.planUnitId)
                  ? 'studyArrangements.collapse'
                  : 'studyArrangements.expand',
                { unit: formatUnitTitle(record.resourceId, record.unitName) },
              )
            "
            :aria-expanded="expanded.includes(record.planUnitId)"
            :disabled="loading"
            @click.stop="toggle(record.planUnitId)"
          >
            <RightOutlined :rotate="expanded.includes(record.planUnitId) ? 90 : 0" />
          </button>
        </template>
        <template v-else-if="column.key === 'book'">
          <span class="book-label" :title="bookLabel(record.bookShortTitle, record.bookTitle)">{{
            bookLabel(record.bookShortTitle, record.bookTitle)
          }}</span>
        </template>
        <template v-else-if="column.key === 'duration'">{{
          durationLabel(record.totalDurationSeconds)
        }}</template>
        <template v-else-if="column.key === 'unit'">
          <div class="unit-heading">
            <div>
              <div class="arrangement-book">
                {{ bookLabel(record.bookShortTitle, record.bookTitle) }}
              </div>
              <span>{{ formatUnitTitle(record.resourceId, record.unitName) }}</span>
            </div>
          </div>
          <div class="mobile-summary">
            <a-tag :color="colors[record.status as keyof typeof colors]">{{
              t(`studyArrangements.${record.status}`)
            }}</a-tag>
            <span>{{ t('studyArrangements.completedCount') }}：{{ record.completedCount }}</span>
            <span
              >{{ t('studyArrangements.duration') }}：{{
                durationLabel(record.totalDurationSeconds)
              }}</span
            >
            <span>{{ t('studyArrangements.next') }}：{{ record.nextReviewDate || '—' }}</span>
          </div>
        </template>
        <a-tag
          v-else-if="column.key === 'status'"
          :color="colors[record.status as keyof typeof colors]"
          >{{ t(`studyArrangements.${record.status}`) }}</a-tag
        >
        <template v-else-if="column.key === 'completed'">{{ record.completedCount }}</template>
        <template v-else-if="column.key === 'next'">{{ record.nextReviewDate || '—' }}</template>
      </template>
    </a-table>
    <a-pagination
      class="arrangement-pagination"
      v-if="!error && (result?.total ?? 0) > 0"
      :current="page"
      :total="result?.total"
      :page-size="10"
      :show-size-changer="false"
      :disabled="loading"
      @change="changePage"
    />
  </div>
</template>

<style scoped>
.arrangements {
  display: flex;
  flex-direction: column;
  min-width: 0;
  color: v-bind('token.colorText');
}
.arrangement-pagination {
  align-self: flex-end;
  margin-top: 14px;
}
.arrangement-table :deep(.ant-table) {
  background: transparent;
  color: v-bind('token.colorText');
  font-size: 13px;
}
.arrangement-table :deep(table),
.arrangement-table :deep(.ant-table-tbody > tr > td) {
  color: v-bind('token.colorText');
}
.arrangement-table :deep(.ant-table-thead > tr > th),
.arrangement-table :deep(.ant-table-tbody > tr > td) {
  padding: 12px;
  border-bottom: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 40%, transparent);
  text-align: left;
}
.arrangement-table :deep(.ant-table-thead > tr > th) {
  background: transparent;
  color: v-bind('token.colorTextSecondary');
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.arrangement-table :deep(.ant-table-thead > tr > th::before) {
  display: none;
}
.arrangement-table :deep(.ant-table-tbody > .arrangement-row:hover > td) {
  background: color-mix(in srgb, v-bind('token.colorFillAlter') 30%, transparent);
}
.arrangement-table :deep(.arrangement-row) {
  cursor: pointer;
}
.arrangement-table :deep(td) {
  overflow-wrap: anywhere;
}
.arrangement-book {
  display: none;
  font-size: 12px;
  color: v-bind('token.colorTextSecondary');
  margin-bottom: 5px;
}
.unit-heading {
  display: flex;
  align-items: center;
  gap: 8px;
}
.book-label {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.unit-heading > div {
  min-width: 0;
}
.arrangement-table :deep(.arrangement-detail-row) {
  background: v-bind('token.colorFillAlter');
}
.expand-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  flex-shrink: 0;
  height: 40px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: v-bind('token.colorText');
  cursor: pointer;
}
.expand-button:hover {
  background: v-bind('token.colorFillAlter');
}
.expand-button:focus-visible {
  outline: 2px solid v-bind('token.colorPrimary');
  outline-offset: 2px;
}
.mobile-summary {
  display: none;
}
@media (max-width: 767px) {
  .arrangement-book {
    display: block;
  }
  .mobile-summary {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 10px;
    font-size: 12px;
  }
  .mobile-summary > span:last-child {
    width: 100%;
  }
  .arrangement-table :deep(.ant-table-cell) {
    padding: 10px 6px;
  }
}
</style>
