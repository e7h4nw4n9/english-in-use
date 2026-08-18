<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { message } from 'ant-design-vue'
import { useAppStore } from '../stores/app'
import { useReaderStore } from '../stores/reader'
import { useReaderAudio } from '../composables/useReaderAudio'
import { useReaderMetadata } from '../composables/useReaderMetadata'
import { useReaderShortcuts } from '../composables/reader/useReaderShortcuts'
import { useReaderTocContext } from '../composables/reader/useReaderTocContext'
import { useStudyTimer, type StudyTimerStopContext } from '../composables/reader/useStudyTimer'
import { useReaderExerciseLoader } from '../composables/reader/useReaderExerciseLoader'
import { useReaderViewportMode } from '../composables/reader/useReaderViewportMode'
import { useReaderOverlayActions } from '../composables/reader/useReaderOverlayActions'
import { useI18n } from 'vue-i18n'
import type { StudySessionUnitRef } from '../types'

// 阅读器子组件。
import ReaderTOC from './reader/ReaderTOC.vue'
import ReaderCanvas from './reader/ReaderCanvas.vue'
import ReaderFooter from './reader/ReaderFooter.vue'
import ReaderAudioPlayer from './reader/ReaderAudioPlayer.vue'
import ReaderExerciseModal from './reader/ReaderExerciseModal.vue'
import ReaderDebugModal from './reader/ReaderDebugModal.vue'

const appStore = useAppStore()
const readerStore = useReaderStore()
const { t } = useI18n()
const { currentBook, config } = storeToRefs(appStore)
const readerRef = ref<HTMLElement | null>(null)
const {
  currentPageLabel,
  viewMode,
  zoomLevel,
  exerciseVisible,
  currentExerciseUrl,
  currentExerciseHtml,
  currentExerciseTitle,
  currentExerciseResourceId,
  showHotspots,
  isUiVisible,
  isSidebarCollapsed,
} = storeToRefs(readerStore)

const {
  metadata,
  loading,
  leftPageUrl,
  rightPageUrl,
  leftPageLabel,
  rightPageLabel,
  sortedPageLabels,
  displayIndex,
  canGoBack,
  canGoForward,
  loadMetadata,
  goBack,
  goForward,
} = useReaderMetadata()

const { toggleAudio, togglePlay, stopAndResetAudio, cleanup: audioCleanup } = useReaderAudio()
const fallbackUnitTitle = computed(() => currentBook.value?.title || '')
const effectiveDebugEnabled = computed(
  () => __DEBUG_FEATURES__ && Boolean(config.value?.system.enable_debug_tools),
)
const exerciseDebugPanelEnabled = computed(() => effectiveDebugEnabled.value)
const {
  currentUnitName,
  currentStudyPlanUnitName,
  currentStudyPlanResourceId,
  currentPageAudioFiles,
} = useReaderTocContext({
  metadata,
  currentPageLabel,
  leftPageLabel,
  rightPageLabel,
  viewMode,
  sortedPageLabels,
  fallbackUnitTitle,
})
const { openExercise } = useReaderExerciseLoader({
  currentBook,
  appStore,
  exerciseVisible,
  currentExerciseUrl,
  currentExerciseHtml,
  currentExerciseTitle,
  currentExerciseResourceId,
  t,
})
const exerciseDebugMeta = computed(() => ({
  productCode: currentBook.value?.product_code || '',
  pageLabel: currentPageLabel.value || '',
  unitName: currentUnitName.value || fallbackUnitTitle.value || '',
}))
const { handleOverlayClick } = useReaderOverlayActions({
  currentBook,
  currentPageLabel,
  openExercise,
  toggleAudio,
})

const { isNarrow, observe, disconnect } = useReaderViewportMode({
  viewMode,
  zoomLevel,
})

const autoStartStudyTimer = computed(() => Boolean(config.value?.system.auto_start_study_timer))
const {
  status: studyTimerStatus,
  isRunning: studyTimerIsRunning,
  formattedDuration: studyTimerFormattedDuration,
  autoPausedByBackground,
  start: startStudyTimer,
  pause: pauseStudyTimer,
  resume: resumeStudyTimer,
  restart: restartStudyTimer,
  reset: resetStudyTimer,
  buildStopContext,
  saveWithAssignedUnit,
  markDiscarded,
} = useStudyTimer({
  productCode: computed(() => currentBook.value?.product_code),
  currentResourceId: currentStudyPlanResourceId,
  currentUnitName: currentStudyPlanUnitName,
  autoStart: autoStartStudyTimer,
})

const saveTimerPromptVisible = ref(false)
const saveInProgress = ref(false)
const pendingStopContext = ref<StudyTimerStopContext | null>(null)
const pendingFlow = ref<'manual' | 'exit' | null>(null)
const pendingAssignedResourceId = ref('')
const shouldResumeRunningOnCancel = ref(false)

const visitedUnitOptions = computed(() => pendingStopContext.value?.visitedUnits || [])
const shouldShowAssignUnitPicker = computed(() => visitedUnitOptions.value.length > 1)

watch(
  currentUnitName,
  (unitName) => {
    readerStore.currentUnitName = unitName
  },
  { immediate: true },
)

watch(currentPageLabel, (newLabel, oldLabel) => {
  if (newLabel !== oldLabel) {
    stopAndResetAudio()
  }
})

watch(
  effectiveDebugEnabled,
  (enabled) => {
    if (!enabled) {
      readerStore.debugVisible = false
    }
  },
  { immediate: true },
)

watch(autoPausedByBackground, (paused, wasPaused) => {
  if (paused && !wasPaused) {
    message.info(t('studyTimer.pausedInBackground'))
  }
})

function handleToggleAudio(path: string) {
  if (currentBook.value) {
    toggleAudio(currentBook.value.product_code, path)
  }
}

/** 捕获一次计时停止流程使用的稳定上下文快照。 */
function captureStopContext(): StudyTimerStopContext | null {
  const wasRunning = studyTimerIsRunning.value
  if (wasRunning) {
    pauseStudyTimer()
  }
  shouldResumeRunningOnCancel.value = wasRunning

  const context = buildStopContext()
  if (!context) {
    resetStudyTimer(true)
    shouldResumeRunningOnCancel.value = false
    return null
  }

  return context
}

function resetPendingFlowState() {
  pendingStopContext.value = null
  pendingFlow.value = null
  pendingAssignedResourceId.value = ''
  shouldResumeRunningOnCancel.value = false
}

/** 用户取消退出时恢复计时器原有运行或暂停状态。 */
function restoreRunningStateOnCancel() {
  if (shouldResumeRunningOnCancel.value) {
    resumeStudyTimer()
  }
  shouldResumeRunningOnCancel.value = false
}

/** 根据用户选择和停止快照确定会话最终归属单元。
 * @param context - 计时停止上下文。
 */
function resolveAssignedUnitFromPicker(context: StudyTimerStopContext): StudySessionUnitRef {
  const matched = context.visitedUnits.find(
    (item) => item.resourceId === pendingAssignedResourceId.value,
  )
  return matched || context.entryUnit
}

/** 将停止上下文保存到选定单元，并处理失败后的恢复。
 * @param context - 计时停止上下文。
 * @param assignedUnit - 用户确认的会话归属单元。
 */
async function persistTimerContext(
  context: StudyTimerStopContext,
  assignedUnit: StudySessionUnitRef,
  closeReaderAfterSave: boolean,
) {
  if (saveInProgress.value) return

  saveInProgress.value = true
  try {
    await appStore.runGlobalLoadingAction(async () => {
      await saveWithAssignedUnit(context, assignedUnit)
      message.success(t('studyTimer.saved'))

      saveTimerPromptVisible.value = false
      resetPendingFlowState()

      if (closeReaderAfterSave) {
        appStore.currentBook = null
      }
    }, t('studyTimer.saving'))
  } catch (error) {
    const errorText = error instanceof Error ? error.message : String(error)
    message.error(t('studyTimer.saveFailed', { error: errorText }))
  } finally {
    saveInProgress.value = false
  }
}

/** 暂停活动计时并打开保存归属确认流程。 */
async function requestStopAndSaveTimer() {
  const context = captureStopContext()
  if (!context) return

  pendingStopContext.value = context
  pendingFlow.value = 'manual'
  pendingAssignedResourceId.value = context.entryUnit.resourceId

  if (shouldShowAssignUnitPicker.value) {
    saveTimerPromptVisible.value = true
    return
  }

  await persistTimerContext(context, context.entryUnit, false)
}

/** 在关闭阅读器前完成必要的计时保存确认。 */
async function handleRequestCloseReader() {
  const context = captureStopContext()
  if (!context) {
    appStore.currentBook = null
    return
  }

  pendingStopContext.value = context
  pendingFlow.value = 'exit'
  pendingAssignedResourceId.value = context.entryUnit.resourceId
  saveTimerPromptVisible.value = true
}

function handleCancelSaveTimerPrompt() {
  saveTimerPromptVisible.value = false
  resetPendingFlowState()
  restoreRunningStateOnCancel()
}

function handleDiscardAndExitReader() {
  markDiscarded()
  saveTimerPromptVisible.value = false
  resetPendingFlowState()
  appStore.currentBook = null
}

/** 使用当前选择的单元保存会话并继续待处理退出动作。 */
async function handleConfirmSaveTimerPrompt() {
  const context = pendingStopContext.value
  if (!context) {
    saveTimerPromptVisible.value = false
    if (pendingFlow.value === 'exit') {
      appStore.currentBook = null
    }
    return
  }

  const assignedUnit = shouldShowAssignUnitPicker.value
    ? resolveAssignedUnitFromPicker(context)
    : context.entryUnit
  await persistTimerContext(context, assignedUnit, pendingFlow.value === 'exit')
}

useReaderShortcuts({
  goBack,
  goForward,
  togglePlayback: togglePlay,
  closeReader: () => {
    void handleRequestCloseReader()
  },
  zoomIn: () => readerStore.zoomIn(),
  zoomOut: () => readerStore.zoomOut(),
  resetZoom: () => readerStore.resetZoom(),
})

onMounted(() => {
  loadMetadata()
  observe(readerRef.value)
  readerStore.showUi()
})

onUnmounted(() => {
  audioCleanup()
  disconnect()
  readerStore.hideUi()
})
</script>

<template>
  <div ref="readerRef" class="reader-view" :class="{ 'reader-ui-hidden': !isUiVisible }">
    <div class="relative flex flex-1 overflow-hidden">
      <Transition name="slide-left">
        <ReaderTOC v-show="isUiVisible && !isSidebarCollapsed" :metadata="metadata" />
      </Transition>

      <ReaderCanvas
        :metadata="metadata"
        :loading="loading"
        :leftPageUrl="leftPageUrl"
        :rightPageUrl="rightPageUrl"
        :leftPageLabel="leftPageLabel"
        :rightPageLabel="rightPageLabel"
        :showHotspots="showHotspots"
        :canGoBack="canGoBack"
        :canGoForward="canGoForward"
        @overlayClick="handleOverlayClick"
        @goBack="goBack"
        @goForward="goForward"
      />
    </div>

    <Transition name="slide-down">
      <ReaderFooter
        v-show="isUiVisible"
        :displayIndex="displayIndex"
        :sortedPageLabels="sortedPageLabels"
        :currentPageAudioFiles="currentPageAudioFiles"
        :currentStudyPlanResourceId="currentStudyPlanResourceId"
        :currentStudyPlanUnitName="currentStudyPlanUnitName"
        :isNarrow="isNarrow"
        :timerStatus="studyTimerStatus"
        :timerDisplay="studyTimerFormattedDuration"
        @toggleAudio="handleToggleAudio"
        @openExercise="openExercise"
        @goBack="goBack"
        @goForward="goForward"
        @requestCloseReader="handleRequestCloseReader"
        @timerStart="startStudyTimer"
        @timerPause="pauseStudyTimer"
        @timerResume="resumeStudyTimer"
        @timerRestart="restartStudyTimer"
        @timerStopSave="requestStopAndSaveTimer"
      />
    </Transition>

    <a-modal
      :open="saveTimerPromptVisible"
      :title="
        pendingFlow === 'exit' ? t('studyTimer.exitConfirmTitle') : t('studyTimer.assignTitle')
      "
      :footer="null"
      :maskClosable="false"
      :closable="false"
      centered
    >
      <p class="mb-4 text-sm text-gray-600 dark:text-gray-300">
        {{
          pendingFlow === 'exit'
            ? t('studyTimer.exitConfirmDescription')
            : t('studyTimer.assignDescription')
        }}
      </p>
      <div v-if="shouldShowAssignUnitPicker" class="mb-4">
        <div class="mb-2 text-sm font-semibold text-gray-600 dark:text-gray-300">
          {{ t('studyTimer.assignLabel') }}
        </div>
        <a-select v-model:value="pendingAssignedResourceId" class="w-full">
          <a-select-option
            v-for="unit in visitedUnitOptions"
            :key="unit.resourceId"
            :value="unit.resourceId"
          >
            <span class="font-semibold">{{ unit.unitName }}</span>
          </a-select-option>
        </a-select>
      </div>
      <div class="flex justify-end gap-2">
        <a-button @click="handleCancelSaveTimerPrompt">
          {{ pendingFlow === 'exit' ? t('studyTimer.cancelExit') : t('common.cancel') }}
        </a-button>
        <a-button v-if="pendingFlow === 'exit'" danger @click="handleDiscardAndExitReader">{{
          t('studyTimer.discardAndExit')
        }}</a-button>
        <a-button type="primary" :loading="saveInProgress" @click="handleConfirmSaveTimerPrompt">{{
          pendingFlow === 'exit' ? t('studyTimer.saveAndExit') : t('studyTimer.confirmSave')
        }}</a-button>
      </div>
    </a-modal>

    <ReaderAudioPlayer />

    <ReaderExerciseModal
      :enableDebugPanel="exerciseDebugPanelEnabled"
      :debugMeta="exerciseDebugMeta"
    />
    <ReaderDebugModal
      v-if="effectiveDebugEnabled"
      :metadata="metadata"
      :sortedPageLabels="sortedPageLabels"
    />
  </div>
</template>

<style>
.reader-view {
  --reader-footer-safe-bottom: constant(safe-area-inset-bottom);
  --reader-footer-safe-bottom: env(safe-area-inset-bottom, 0px);
  --reader-footer-dock-bottom: 16px;
  --reader-footer-height: 54px;
  --reader-footer-clearance: calc(
    var(--reader-footer-dock-bottom) + var(--reader-footer-safe-bottom) +
      var(--reader-footer-height) + 24px
  );
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #f3f4f6;
}

.reader-view.reader-ui-hidden {
  --reader-footer-dock-bottom: 0px;
  --reader-footer-height: 0px;
  --reader-footer-clearance: 8px;
}

html.dark .reader-view {
  background: #111827;
}

@media (max-width: 1024px) {
  .reader-view {
    --reader-footer-dock-bottom: 12px;
    --reader-footer-height: 48px;
  }
}

@supports (-webkit-touch-callout: none) {
  @media (hover: none) and (pointer: coarse) {
    .reader-view {
      --reader-footer-dock-bottom: 18px;
      --reader-footer-height: 60px;
    }
  }

  @media (hover: none) and (pointer: coarse) and (max-width: 1024px) {
    .reader-view {
      --reader-footer-dock-bottom: 14px;
      --reader-footer-height: 54px;
    }
  }
}

/* 阅读器搜索和目录树使用的全局样式。 */
.modern-executive-search .ant-input {
  border-radius: 24px !important;
  background-color: rgba(0, 0, 0, 0.025) !important;
  border: 1px solid rgba(0, 0, 0, 0.04) !important;
  font-size: 14px !important;
  padding: 8px 16px !important;
}
.dark .modern-executive-search .ant-input {
  background-color: rgba(255, 255, 255, 0.025) !important;
  border: 1px solid rgba(255, 255, 255, 0.06) !important;
}
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.05);
  border-radius: 10px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.05);
}

.slide-right-enter-active,
.slide-right-leave-active {
  transition: all 0.3s ease;
}
.slide-right-enter-from,
.slide-right-leave-to {
  transform: translateX(100%);
  opacity: 0;
}

.slide-left-enter-active,
.slide-left-leave-active {
  transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}
.slide-left-enter-from,
.slide-left-leave-to {
  transform: translateX(-100%);
  opacity: 0;
}

.slide-down-enter-active,
.slide-down-leave-active {
  transition: all 0.3s ease;
}
.slide-down-enter-from,
.slide-down-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>
