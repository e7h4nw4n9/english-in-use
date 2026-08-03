<script setup lang="ts">
import { LeftOutlined, RightOutlined } from '@ant-design/icons-vue'
import DailyTaskPanel from './DailyTaskPanel.vue'
import { useStudyPlanPage } from '../../composables/study-plan/useStudyPlanPage'

const {
  t,
  token,
  viewModes,
  viewMode,
  setViewMode,
  periodLabel,
  goPrev,
  goNext,
  goToday,
  loadingSummary,
  weekdays,
  weekdayKeys,
  monthCellDates,
  weekDates,
  selectedDate,
  getDateCellStyle,
  getDateDayNumber,
  isTodayDate,
  isCurrentMonthDate,
  summaryOf,
  selectWeekDate,
  openMonthDrawer,
  groupedTasksByDate,
  isDateTasksLoading,
  completingTaskId,
  jumpToStudy,
  markTaskDone,
  drawerOpen,
  selectedDateLabel,
} = useStudyPlanPage()
</script>

<template>
  <section class="study-plan-page h-full w-full">
    <div class="study-plan-shell">
      <div class="study-card study-toolbar">
        <div class="mode-switch" role="tablist" :aria-label="t('app.homeTabs.studyPlan')">
          <button
            v-for="mode in viewModes"
            :key="mode"
            type="button"
            class="mode-btn"
            :class="{ active: mode === viewMode }"
            :aria-selected="mode === viewMode"
            @click="setViewMode(mode)"
          >
            {{ t(`studyPlan.view.${mode}`) }}
          </button>
        </div>

        <div class="period-nav">
          <a-button shape="circle" aria-label="Previous period" @click="goPrev">
            <template #icon><LeftOutlined /></template>
          </a-button>
          <div class="period-label">{{ periodLabel }}</div>
          <a-button shape="circle" aria-label="Next period" @click="goNext">
            <template #icon><RightOutlined /></template>
          </a-button>
          <a-button type="default" class="today-btn" @click="goToday">
            {{ t('studyPlan.nav.today') }}
          </a-button>
        </div>
      </div>

      <div class="study-card study-grid">
        <div v-if="loadingSummary" class="loading-state">
          {{ t('app.loading') }}
        </div>

        <template v-else>
          <div class="weekday-row">
            <div
              v-for="(day, index) in weekdays"
              :key="`wk-${weekdayKeys[index]}`"
              class="weekday-cell"
            >
              {{ day }}
            </div>
          </div>

          <template v-if="viewMode === 'month'">
            <div class="date-grid-month">
              <button
                v-for="date in monthCellDates"
                :key="date"
                type="button"
                class="date-cell"
                :class="{
                  'is-selected': selectedDate === date,
                  'is-muted': !isCurrentMonthDate(date),
                }"
                :style="getDateCellStyle(date, selectedDate === date)"
                @click="openMonthDrawer(date)"
              >
                <div class="date-cell-head">
                  <span class="date-number">{{ getDateDayNumber(date) }}</span>
                  <span v-if="isTodayDate(date)" class="today-tag">
                    {{ t('studyPlan.nav.today') }}
                  </span>
                </div>
                <div class="task-stats-list">
                  <div class="stat-item completed">
                    <span class="stat-label">{{ t('studyPlan.done') }}</span>
                    <span class="stat-value">{{ summaryOf(date)?.completed || 0 }}</span>
                  </div>
                  <div class="stat-item overdue">
                    <span class="stat-label">{{ t('studyPlan.overdue') }}</span>
                    <span class="stat-value">{{ summaryOf(date)?.overdue || 0 }}</span>
                  </div>
                  <div class="stat-item total">
                    <span class="stat-label">{{ t('studyPlan.total') }}</span>
                    <span class="stat-value">{{ summaryOf(date)?.total || 0 }}</span>
                  </div>
                </div>
              </button>
            </div>
          </template>

          <template v-else>
            <div class="date-grid-week">
              <button
                v-for="date in weekDates"
                :key="date"
                type="button"
                class="date-cell"
                :class="{ 'is-selected': selectedDate === date }"
                :style="getDateCellStyle(date, selectedDate === date)"
                @click="selectWeekDate(date)"
              >
                <div class="date-cell-head">
                  <span class="date-number">{{ getDateDayNumber(date) }}</span>
                  <span v-if="isTodayDate(date)" class="today-tag">
                    {{ t('studyPlan.nav.today') }}
                  </span>
                </div>
                <div class="task-stats-list">
                  <div class="stat-item completed">
                    <span class="stat-label">{{ t('studyPlan.done') }}</span>
                    <span class="stat-value">{{ summaryOf(date)?.completed || 0 }}</span>
                  </div>
                  <div class="stat-item overdue">
                    <span class="stat-label">{{ t('studyPlan.overdue') }}</span>
                    <span class="stat-value">{{ summaryOf(date)?.overdue || 0 }}</span>
                  </div>
                  <div class="stat-item total">
                    <span class="stat-label">{{ t('studyPlan.total') }}</span>
                    <span class="stat-value">{{ summaryOf(date)?.total || 0 }}</span>
                  </div>
                </div>
              </button>
            </div>
          </template>
        </template>
      </div>

      <div v-if="viewMode === 'week'" class="study-card week-task-panel">
        <div class="week-panel-head">
          {{ t('studyPlan.taskListFor', { date: selectedDateLabel }) }}
        </div>
        <DailyTaskPanel
          :grouped-tasks="groupedTasksByDate[selectedDate] || []"
          :loading="isDateTasksLoading(selectedDate)"
          :show-actions="isTodayDate(selectedDate)"
          :completing-task-id="completingTaskId"
          @jump-to-study="jumpToStudy"
          @mark-task-done="markTaskDone"
        />
      </div>

      <a-drawer
        v-model:open="drawerOpen"
        :title="t('studyPlan.taskListFor', { date: selectedDateLabel })"
        placement="bottom"
        :height="'70vh'"
        destroy-on-close
      >
        <DailyTaskPanel
          :grouped-tasks="groupedTasksByDate[selectedDate] || []"
          :loading="isDateTasksLoading(selectedDate)"
          :show-actions="isTodayDate(selectedDate)"
          :completing-task-id="completingTaskId"
          @jump-to-study="jumpToStudy"
          @mark-task-done="markTaskDone"
        />
      </a-drawer>
    </div>
  </section>
</template>

<style scoped>
.study-plan-page {
  box-sizing: border-box;
  position: relative;
  isolation: isolate;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  overflow-y: auto;
  overflow-x: hidden;
  min-height: 100%;
  padding: 16px;
}

.study-plan-shell {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  min-height: 100%;
}

.study-card {
  width: 100%;
  max-width: 100%;
  min-width: 0;
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 75%, transparent);
  border-radius: 16px;
  background: v-bind('token.colorBgContainer');
}

.study-toolbar {
  padding: 14px 16px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.mode-switch {
  display: inline-flex;
  gap: 4px;
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 75%, transparent);
  border-radius: 999px;
  padding: 4px;
  background: color-mix(in srgb, v-bind('token.colorBgElevated') 88%, transparent);
}

.mode-btn {
  border: 0;
  background: transparent;
  color: v-bind('token.colorTextSecondary');
  font-size: 13px;
  font-weight: 600;
  line-height: 1;
  padding: 8px 16px;
  border-radius: 999px;
  cursor: pointer;
  transition:
    color 0.2s ease,
    background-color 0.2s ease;
}

.mode-btn:hover {
  color: v-bind('token.colorText');
}

.mode-btn.active {
  color: v-bind('token.colorPrimary');
  background: color-mix(in srgb, v-bind('token.colorPrimaryBg') 85%, transparent);
}

.mode-btn:focus-visible {
  outline: 2px solid v-bind('token.colorPrimary');
  outline-offset: 2px;
}

.period-nav {
  display: inline-flex;
  flex-wrap: wrap;
  max-width: 100%;
  align-items: center;
  gap: 12px;
}

.period-label {
  min-width: 0;
  flex: 1 1 140px;
  text-align: center;
  font-size: 15px;
  font-weight: 700;
  color: v-bind('token.colorText');
}

.today-btn {
  flex-shrink: 0;
}

.study-grid {
  padding: 16px;
}

.weekday-row {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 12px;
  margin-bottom: 12px;
}

.weekday-cell {
  text-align: center;
  font-size: 12px;
  line-height: 1.3;
  font-weight: 700;
  color: v-bind('token.colorTextTertiary');
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.date-grid-month,
.date-grid-week {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 12px;
}

.date-cell {
  cursor: pointer;
  position: relative;
  border: 1px solid transparent;
  border-radius: 16px;
  padding: 12px;
  text-align: left;
  min-height: 128px;
  width: 100%;
  display: flex;
  flex-direction: column;
  transition:
    border-color 0.2s ease,
    box-shadow 0.2s ease,
    transform 0.2s ease;
}

.date-cell:hover {
  transform: translateY(-2px);
  border-color: color-mix(in srgb, v-bind('token.colorPrimaryBorder') 78%, transparent);
  box-shadow: 0 12px 20px -12px color-mix(in srgb, v-bind('token.colorPrimary') 30%, transparent);
}

.date-cell.is-selected {
  box-shadow: 0 0 0 1px color-mix(in srgb, v-bind('token.colorPrimary') 30%, transparent);
}

.date-cell.is-muted .date-number {
  opacity: 0.5;
}

.date-cell-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 8px;
}

.date-number {
  font-size: 18px;
  line-height: 1;
  font-weight: 800;
  color: v-bind('token.colorTextHeading');
}

.today-tag {
  position: absolute;
  top: 0;
  right: 0;
  font-size: 9px;
  font-weight: 800;
  color: #ffffff;
  background: v-bind('token.colorInfo');
  padding: 3px 8px;
  border-radius: 0 16px 0 16px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  z-index: 1;
}

.task-stats-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: auto;
}

.stat-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  line-height: 1.4;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
}

.stat-label {
  opacity: 0.85;
}

.stat-value {
  font-weight: 800;
}

.stat-item.completed {
  color: v-bind('token.colorSuccess');
  background: color-mix(in srgb, v-bind('token.colorSuccessBg') 70%, transparent);
}

.stat-item.overdue {
  color: v-bind('token.colorError');
  background: color-mix(in srgb, v-bind('token.colorErrorBg') 75%, transparent);
}

.stat-item.total {
  color: v-bind('token.colorPrimary');
  background: color-mix(in srgb, v-bind('token.colorPrimaryBg') 75%, transparent);
}

.week-task-panel {
  padding: 16px;
}

.week-panel-head {
  margin-bottom: 12px;
  font-size: 15px;
  line-height: 1.3;
  font-weight: 700;
  color: v-bind('token.colorTextHeading');
}

.loading-state {
  padding: 32px 0;
  text-align: center;
  font-size: 14px;
  line-height: 1.4;
  color: v-bind('token.colorTextSecondary');
}
</style>
<style scoped src="./StudyPlanPage.responsive.css"></style>
