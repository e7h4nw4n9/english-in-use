<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '../../stores/app'
import { useReaderStore } from '../../stores/reader'
import {
  LeftOutlined,
  RightOutlined,
  UnorderedListOutlined,
  EyeOutlined,
  EyeInvisibleOutlined,
  FullscreenExitOutlined,
  FileTextOutlined,
  BlockOutlined,
  HomeOutlined,
  CalendarOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons-vue'
import { message, theme } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import type {
  OverlayAudio,
  ExerciseInfo,
  StudyPlanStatusResponse,
  StudyPlanUpsertResponse,
} from '../../types'
import { abandonStudyPlan, getStudyPlanStatus, upsertStudyPlan } from '../../lib/api/studyPlan'
import ReaderStudyTimerFloat from './ReaderStudyTimerFloat.vue'

const { useToken } = theme
const { token } = useToken()
const { t } = useI18n()

const props = defineProps<{
  displayIndex: number
  sortedPageLabels: string[]
  currentPageAudioFiles: OverlayAudio[]
  currentStudyPlanResourceId: string | null
  currentStudyPlanUnitName: string
  timerStatus: 'idle' | 'running' | 'paused'
  timerDisplay: string
  isNarrow?: boolean
}>()

const emit = defineEmits<{
  (e: 'toggleAudio', path: string): void
  (e: 'openExercise', ex: ExerciseInfo): void
  (e: 'goBack'): void
  (e: 'goForward'): void
  (e: 'requestCloseReader'): void
  (e: 'timerStart'): void
  (e: 'timerPause'): void
  (e: 'timerResume'): void
  (e: 'timerRestart'): void
  (e: 'timerStopSave'): void
}>()

const appStore = useAppStore()
const readerStore = useReaderStore()
const { viewMode, showHotspots, isSidebarCollapsed } = storeToRefs(readerStore)
const { currentBook } = storeToRefs(appStore)
const studyPlanStatus = ref<StudyPlanStatusResponse | null>(null)
const studyPlanLoading = ref(false)
const studyPlanBusy = ref(false)
const statusRefreshTimer = ref<ReturnType<typeof setTimeout> | null>(null)
const statusRequestSeq = ref(0)

interface StudyPlanContext {
  productCode: string | null
  resourceId: string | null
  unitName: string
}

const currentRangeText = computed(() => {
  const left = props.sortedPageLabels[props.displayIndex] || ''
  if (viewMode.value === 'single') {
    return left
  }
  const right = props.sortedPageLabels[props.displayIndex + 1] || ''
  if (!right) return left
  return `${left}-${right}`
})

const timerPanelVisible = ref(false)
const canStartTimerFromIsland = computed(() => props.timerStatus === 'idle')
const timerIslandTooltip = computed(() =>
  canStartTimerFromIsland.value ? t('studyTimer.start') : t('studyTimer.title'),
)

function startTimerFromIsland() {
  if (!canStartTimerFromIsland.value) return
  emit('timerStart')
}

function toggleSidebar() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value
}

function toggleViewMode() {
  if (viewMode.value === 'spread') {
    viewMode.value = 'single'
    return
  }

  viewMode.value = 'spread'
}

const canUseStudyPlan = computed(() =>
  Boolean(currentBook.value && props.currentStudyPlanResourceId),
)
const isStudyPlanActive = computed(() => {
  if (!canUseStudyPlan.value) return false
  const status = studyPlanStatus.value
  if (!status || !status.inPlan) return false
  return status.planStatus !== null && status.planStatus !== 2
})

const studyPlanTooltip = computed(() => {
  if (!currentBook.value) return t('studyPlan.unavailable')
  if (!props.currentStudyPlanResourceId) return t('studyPlan.unavailable')
  if (studyPlanLoading.value) return t('studyPlan.loading')

  if (isStudyPlanActive.value) {
    const nextDate = studyPlanStatus.value?.nextReviewDate
    if (nextDate) return t('studyPlan.nextReview', { date: nextDate })
    return t('studyPlan.inPlan')
  }
  return t('studyPlan.add')
})

function getStudyPlanContext(): StudyPlanContext {
  return {
    productCode: currentBook.value?.product_code ?? null,
    resourceId: props.currentStudyPlanResourceId ?? null,
    unitName: props.currentStudyPlanUnitName || '',
  }
}

function isStudyPlanContextActive(context: StudyPlanContext) {
  const currentContext = getStudyPlanContext()
  return (
    context.productCode === currentContext.productCode &&
    context.resourceId === currentContext.resourceId &&
    context.unitName === currentContext.unitName
  )
}

function invalidateStudyPlanStatusSync() {
  statusRequestSeq.value += 1
  if (statusRefreshTimer.value) {
    clearTimeout(statusRefreshTimer.value)
    statusRefreshTimer.value = null
  }
}

function resetStudyPlanStateForContextChange() {
  invalidateStudyPlanStatusSync()
  studyPlanStatus.value = null
  studyPlanLoading.value = canUseStudyPlan.value
}

async function refreshStudyPlanStatus(setLoadingOnStart = true) {
  const context = getStudyPlanContext()
  if (!context.productCode || !context.resourceId) {
    studyPlanStatus.value = null
    studyPlanLoading.value = false
    return
  }
  const requestSeq = statusRequestSeq.value + 1
  statusRequestSeq.value = requestSeq
  if (setLoadingOnStart) studyPlanLoading.value = true

  try {
    const nextStatus = await getStudyPlanStatus(context.productCode, context.resourceId)
    if (requestSeq !== statusRequestSeq.value || !isStudyPlanContextActive(context)) {
      return
    }
    studyPlanStatus.value = nextStatus
  } catch {
    if (requestSeq !== statusRequestSeq.value || !isStudyPlanContextActive(context)) {
      return
    }
    studyPlanStatus.value = null
  } finally {
    if (requestSeq === statusRequestSeq.value) {
      studyPlanLoading.value = false
    }
  }
}

function queueRefreshStudyPlanStatus(delayMs = 120, setLoadingOnStart = false) {
  if (statusRefreshTimer.value) clearTimeout(statusRefreshTimer.value)
  statusRefreshTimer.value = setTimeout(() => {
    statusRefreshTimer.value = null
    void refreshStudyPlanStatus(setLoadingOnStart)
  }, delayMs)
}

function applyUpsertResult(result: StudyPlanUpsertResponse) {
  studyPlanStatus.value = {
    inPlan: true,
    planStatus: result.planStatus,
    planUnitId: result.planUnitId,
    completedStages: result.completedStages,
    nextReviewDate: result.nextReviewDate,
    overdueCount: studyPlanStatus.value?.overdueCount ?? 0,
  }
}

async function toggleStudyPlan() {
  if (!currentBook.value || !props.currentStudyPlanResourceId || studyPlanBusy.value) return

  studyPlanBusy.value = true
  try {
    if (!isStudyPlanActive.value) {
      const upsertResult = await upsertStudyPlan(
        currentBook.value.product_code,
        props.currentStudyPlanResourceId,
        props.currentStudyPlanUnitName || currentBook.value.title,
      )
      applyUpsertResult(upsertResult)
      message.success(t('studyPlan.added'))
    } else {
      await abandonStudyPlan(currentBook.value.product_code, props.currentStudyPlanResourceId)
      studyPlanStatus.value = {
        inPlan: true,
        planStatus: 2,
        planUnitId: studyPlanStatus.value?.planUnitId ?? null,
        completedStages: studyPlanStatus.value?.completedStages ?? [],
        nextReviewDate: null,
        overdueCount: studyPlanStatus.value?.overdueCount ?? 0,
      }
      message.success(t('studyPlan.abandoned'))
    }
    queueRefreshStudyPlanStatus(240, false)
  } catch (error) {
    const errorText = error instanceof Error ? error.message : String(error)
    message.error(t('studyPlan.actionFailed', { error: errorText }))
  } finally {
    studyPlanBusy.value = false
  }
}

watch(
  [
    () => currentBook.value?.product_code,
    () => props.currentStudyPlanResourceId,
    () => props.currentStudyPlanUnitName,
  ],
  () => {
    resetStudyPlanStateForContextChange()
    if (!canUseStudyPlan.value) return
    queueRefreshStudyPlanStatus(120, false)
  },
  { immediate: true },
)

watch(
  () => props.timerStatus,
  (status) => {
    timerPanelVisible.value = status !== 'idle'
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  invalidateStudyPlanStatusSync()
})
</script>

<template>
  <div class="reader-footer-floating">
    <nav class="reader-dock" role="toolbar" aria-label="Reader controls">
      <a-tooltip placement="top" :title="t('reader.home')">
        <a-button type="text" class="dock-btn" @click="emit('requestCloseReader')">
          <template #icon><HomeOutlined /></template>
        </a-button>
      </a-tooltip>

      <a-tooltip placement="top" :title="t('reader.toc')">
        <a-button type="text" class="dock-btn" @click="toggleSidebar">
          <template #icon><UnorderedListOutlined /></template>
        </a-button>
      </a-tooltip>

      <span class="dock-divider" aria-hidden="true"></span>

      <a-tooltip placement="top" :title="t('reader.prevPage')">
        <a-button type="text" class="dock-btn" @click="emit('goBack')">
          <template #icon><LeftOutlined /></template>
        </a-button>
      </a-tooltip>

      <div class="range-chip" :title="currentRangeText">
        {{ currentRangeText }} <span class="range-divider">/</span>
        {{ sortedPageLabels.length }}
      </div>

      <a-tooltip placement="top" :title="t('reader.nextPage')">
        <a-button type="text" class="dock-btn" @click="emit('goForward')">
          <template #icon><RightOutlined /></template>
        </a-button>
      </a-tooltip>

      <a-tooltip
        placement="top"
        :title="viewMode === 'single' ? t('reader.viewSpread') : t('reader.viewSingle')"
      >
        <a-button type="text" class="dock-btn" :disabled="isNarrow" @click="toggleViewMode">
          <template #icon>
            <BlockOutlined v-if="viewMode === 'single'" />
            <FileTextOutlined v-else />
          </template>
        </a-button>
      </a-tooltip>

      <span class="dock-divider" aria-hidden="true"></span>

      <a-tooltip placement="top" :title="timerIslandTooltip">
        <a-button
          type="text"
          class="dock-btn"
          :disabled="!canStartTimerFromIsland"
          @click="startTimerFromIsland"
        >
          <template #icon><ClockCircleOutlined /></template>
        </a-button>
      </a-tooltip>

      <a-tooltip placement="top" :title="studyPlanTooltip">
        <a-button
          type="text"
          class="dock-btn"
          :class="{ active: isStudyPlanActive }"
          :disabled="!canUseStudyPlan || studyPlanBusy"
          @click="toggleStudyPlan"
        >
          <template #icon>
            <CheckCircleOutlined v-if="isStudyPlanActive" />
            <CalendarOutlined v-else />
          </template>
        </a-button>
      </a-tooltip>

      <a-tooltip
        placement="top"
        :title="showHotspots ? t('reader.hideHotspots') : t('reader.showHotspots')"
      >
        <a-button
          type="text"
          class="dock-btn"
          :class="{ active: showHotspots }"
          @click="showHotspots = !showHotspots"
        >
          <template #icon>
            <EyeOutlined v-if="showHotspots" />
            <EyeInvisibleOutlined v-else />
          </template>
        </a-button>
      </a-tooltip>

      <a-tooltip placement="top" :title="t('reader.resetZoom')">
        <a-button type="text" class="dock-btn" @click="readerStore.resetZoom()">
          <template #icon><FullscreenExitOutlined /></template>
        </a-button>
      </a-tooltip>
    </nav>

    <ReaderStudyTimerFloat
      :visible="timerPanelVisible"
      :timerStatus="timerStatus"
      :timerDisplay="timerDisplay"
      @timerStart="emit('timerStart')"
      @timerPause="emit('timerPause')"
      @timerResume="emit('timerResume')"
      @timerRestart="emit('timerRestart')"
      @timerStopSave="emit('timerStopSave')"
    />
  </div>
</template>

<style scoped>
.reader-footer-floating {
  pointer-events: none;
}

.reader-footer-floating :deep(.ant-btn),
.reader-footer-floating :deep(.study-timer-float) {
  pointer-events: auto;
}

.reader-dock {
  pointer-events: auto;
  position: fixed;
  left: 50%;
  bottom: calc(var(--reader-footer-dock-bottom, 16px) + var(--reader-footer-safe-bottom, 0px));
  z-index: 1000;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  min-height: var(--reader-footer-height, 54px);
  width: fit-content;
  max-width: 96vw;
  /* 液态玻璃核心样式。 */
  background: color-mix(in srgb, v-bind('token.colorBgElevated') 70%, transparent);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border: 1px solid color-mix(in srgb, v-bind('token.colorWhite') 10%, transparent);
  border-top: 1px solid color-mix(in srgb, v-bind('token.colorWhite') 25%, transparent);
  border-radius: 16px;
  box-shadow:
    0 10px 40px -10px rgba(0, 0, 0, 0.3),
    inset 0 0 0 1px color-mix(in srgb, v-bind('token.colorWhite') 5%, transparent);

  overflow-x: auto;
  scrollbar-width: none;
  touch-action: manipulation;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.reader-dock::-webkit-scrollbar {
  display: none;
}

:deep(.dock-btn.ant-btn) {
  min-width: 40px;
  height: 40px;
  border-radius: 10px;
  color: v-bind('token.colorTextSecondary');
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  background: transparent;
  border: none;
  padding: 0;
  transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}

:deep(.dock-btn.ant-btn:not(:disabled):hover) {
  color: v-bind('token.colorPrimary');
  background: color-mix(in srgb, v-bind('token.colorPrimaryBg') 45%, transparent);
  transform: translateY(-1px);
}

:deep(.dock-btn.ant-btn:not(:disabled):active) {
  transform: scale(0.92);
}

.dock-divider {
  width: 1px;
  height: 20px;
  margin: 0 2px;
  background: color-mix(in srgb, v-bind('token.colorTextQuaternary') 25%, transparent);
}

.range-chip {
  min-width: 90px;
  height: 34px;
  padding: 0 10px;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 600;
  line-height: 1;
  font-variant-numeric: tabular-nums;
  color: v-bind('token.colorText');
  background: color-mix(in srgb, v-bind('token.colorFillSecondary') 40%, transparent);
  border: 1px solid color-mix(in srgb, v-bind('token.colorWhite') 8%, transparent);
  white-space: nowrap;
}

.range-divider {
  opacity: 0.45;
}

@media (max-width: 1024px) {
  .reader-dock {
    gap: 4px;
    padding: 4px 6px;
    min-height: var(--reader-footer-height, 48px);
  }

  :deep(.dock-btn.ant-btn) {
    min-width: 38px;
    height: 38px;
    font-size: 15px;
  }

  .range-chip {
    min-width: 80px;
    height: 32px;
    font-size: 11px;
    padding: 0 8px;
  }
}

@supports (-webkit-touch-callout: none) {
  @media (hover: none) and (pointer: coarse) {
    .reader-dock {
      gap: 8px;
      padding: 8px 10px;
      min-height: var(--reader-footer-height, 60px);
    }

    :deep(.dock-btn.ant-btn) {
      min-width: 44px;
      height: 44px;
      border-radius: 12px;
      font-size: 18px;
    }

    .dock-divider {
      height: 22px;
      margin: 0 3px;
    }

    .range-chip {
      min-width: 96px;
      height: 38px;
      padding: 0 12px;
      font-size: 13px;
    }
  }

  @media (hover: none) and (pointer: coarse) and (max-width: 1024px) {
    .reader-dock {
      gap: 6px;
      padding: 7px 9px;
      min-height: var(--reader-footer-height, 54px);
    }

    :deep(.dock-btn.ant-btn) {
      min-width: 42px;
      height: 42px;
      font-size: 17px;
    }

    .range-chip {
      min-width: 88px;
      height: 36px;
      padding: 0 10px;
      font-size: 12px;
    }
  }
}

@media (prefers-reduced-motion: reduce) {
  :deep(.dock-btn.ant-btn) {
    transition: none;
  }
}
</style>
