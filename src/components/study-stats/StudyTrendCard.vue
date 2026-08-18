<script setup lang="ts">
import { computed } from 'vue'
import { theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import type { StudyStatsPeriodType, StudyStatsTrendItem } from '@/types'
import {
  getDisplayDurationMinutes,
  getTrendAxisLabelPosition,
  getTrendAxisScale,
  getTrendHeight,
} from './trendAxis'

const props = defineProps<{
  periodType: StudyStatsPeriodType
  trend: StudyStatsTrendItem[]
}>()

const emit = defineEmits<{
  (event: 'select', item: StudyStatsTrendItem): void
}>()

const { t } = useI18n()
const { token } = theme.useToken()
const maxDuration = computed(() => Math.max(0, ...props.trend.map((item) => item.duration)))
const axisScale = computed(() => getTrendAxisScale(maxDuration.value))

/** 根据统计周期格式化趋势横轴标签。
 * @param date - 趋势数据对应的日期或月份。
 */
function formatLabel(date: string): string {
  if (props.periodType === 'year') return `${Number(date.slice(5, 7))}月`
  if (props.periodType === 'month') return date.slice(8)
  return date.slice(5)
}

/** 计算趋势柱高度。
 * @param duration - 学习时长，单位为秒。
 */
function trendHeight(duration: number): string {
  return getTrendHeight(duration, axisScale.value.maxMinutes)
}
</script>

<template>
  <article class="stats-card trend-card">
    <header class="card-title">{{ t('studyStats.trend') }}</header>
    <div v-if="trend.length === 0" class="empty-state">{{ t('studyStats.empty') }}</div>
    <div v-else class="trend-container">
      <div class="trend-y-axis">
        <span
          v-for="label in axisScale.labels"
          :key="label"
          class="y-label"
          :style="{ bottom: getTrendAxisLabelPosition(label, axisScale.maxMinutes) }"
        >
          {{ label }}
        </span>
      </div>
      <div class="trend-bars-wrapper">
        <div class="trend-bars">
          <button
            v-for="item in trend"
            :key="item.date"
            type="button"
            class="trend-bar-item"
            :class="{ clickable: item.duration > 0 }"
            :disabled="item.duration <= 0 || periodType === 'year'"
            @click="emit('select', item)"
          >
            <span class="trend-bar-track">
              <span
                v-if="item.duration > 0"
                class="trend-bar-value"
                :style="{ bottom: `calc(${trendHeight(item.duration)} + 4px)` }"
              >
                {{ getDisplayDurationMinutes(item.duration) }}
              </span>
              <span class="trend-bar-fill" :style="{ height: trendHeight(item.duration) }"></span>
            </span>
            <span class="trend-bar-label">{{ formatLabel(item.date) }}</span>
          </button>
        </div>
      </div>
    </div>
  </article>
</template>

<style scoped>
.stats-card {
  max-width: 100%;
  min-width: 0;
  border: 1px solid color-mix(in srgb, v-bind('token.colorBorderSecondary') 50%, transparent);
  border-radius: 12px;
  padding: 14px;
  background: v-bind('token.colorBgContainer');
}

.card-title {
  font-size: 15px;
  font-weight: 700;
  margin-bottom: 10px;
  color: v-bind('token.colorTextHeading');
}

.empty-state {
  min-height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: v-bind('token.colorTextSecondary');
}

.trend-container {
  --trend-plot-height: 190px;
  --trend-x-label-offset: 15px;
  display: flex;
  gap: 12px;
  height: 220px;
}

.trend-y-axis {
  position: relative;
  align-self: flex-end;
  height: var(--trend-plot-height);
  margin-bottom: var(--trend-x-label-offset);
  font-size: 10px;
  color: v-bind('token.colorTextTertiary');
  text-align: right;
  min-width: 40px;
}

.y-label {
  position: absolute;
  right: 0;
  line-height: 1;
  transform: translateY(50%);
}

.trend-bars-wrapper {
  flex: 1;
  height: 100%;
  overflow-x: auto;
  overflow-y: hidden;
}

.trend-bars {
  display: flex;
  align-items: flex-end;
  gap: v-bind("periodType === 'year' ? '12px' : '2px'");
  height: 100%;
  width: 100%;
  min-width: min-content;
}

.trend-bar-item {
  flex: 1;
  min-width: v-bind("periodType === 'year' ? '40px' : (periodType === 'month' ? '18px' : '24px')");
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  border: 0;
  padding: 0;
  background: transparent;
  transition: opacity 0.2s;
}

.trend-bar-item.clickable:not(:disabled) {
  cursor: pointer;
}

.trend-bar-item.clickable:not(:disabled):hover {
  opacity: 0.8;
}

.trend-bar-track {
  position: relative;
  width: 100%;
  height: var(--trend-plot-height);
  border-radius: 6px;
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 30%, transparent);
  display: flex;
  align-items: flex-end;
}

.trend-bar-fill {
  width: 100%;
  border-radius: 6px;
  background: linear-gradient(to top, v-bind('token.colorPrimary'), v-bind('token.colorInfo'));
}

.trend-bar-label,
.trend-bar-value {
  font-size: 10px;
  line-height: 1;
  color: v-bind('token.colorTextTertiary');
  white-space: nowrap;
}

.trend-bar-label {
  font-size: 11px;
  color: v-bind('token.colorTextSecondary');
}

.trend-bar-value {
  position: absolute;
  left: 50%;
  z-index: 1;
  transform: translateX(-50%);
  font-variant-numeric: tabular-nums;
  pointer-events: none;
}

@media (max-width: 960px) {
  .trend-y-axis {
    flex: 0 0 28px;
    min-width: 28px;
    font-size: 9px;
  }

  .trend-container {
    gap: 6px;
  }
}
</style>
