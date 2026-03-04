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
  AppstoreOutlined,
  CalendarOutlined,
  CheckCircleOutlined,
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

const { useToken } = theme
const { token } = useToken()
const { t } = useI18n()

const props = defineProps<{
  displayIndex: number
  sortedPageLabels: string[]
  currentPageAudioFiles: OverlayAudio[]
  currentStudyPlanResourceId: string | null
  currentStudyPlanUnitName: string
  isNarrow?: boolean
}>()

const emit = defineEmits<{
  (e: 'toggleAudio', path: string): void
  (e: 'openExercise', ex: ExerciseInfo): void
  (e: 'goBack'): void
  (e: 'goForward'): void
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

function closeReader() {
  appStore.currentBook = null
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

onBeforeUnmount(() => {
  invalidateStudyPlanStatusSync()
})
</script>

<template>
  <div class="reader-footer-floating">
    <!-- 1. Integrated Page Indicator & Navigation (Bottom Center) -->
    <div
      class="pointer-events-auto fixed bottom-6 left-1/2 z-[1000] flex -translate-x-1/2 items-center gap-3 rounded-full border px-3 py-1.5 shadow-lg backdrop-blur-md transition-all duration-300"
      :style="{
        backgroundColor: token.colorBgElevated + 'aa',
        borderColor: token.colorBorderSecondary,
        color: token.colorTextSecondary,
      }"
    >
      <!-- Prev Button -->
      <a-button
        type="text"
        size="small"
        class="flex h-auto items-center p-0 text-inherit opacity-60 transition-opacity hover:opacity-100"
        :title="t('reader.prevPage')"
        @click="emit('goBack')"
      >
        <template #icon><LeftOutlined /></template>
      </a-button>

      <!-- Page Range Text -->
      <div class="min-w-[60px] px-1 text-center text-xs font-bold tabular-nums tracking-wider">
        {{ currentRangeText }} <span class="mx-0.5 opacity-40">/</span>
        {{ sortedPageLabels.length }}
      </div>

      <!-- View Mode Toggle -->
      <a-button
        type="text"
        size="small"
        class="flex h-auto items-center p-0 text-inherit opacity-60 transition-opacity hover:opacity-100"
        :disabled="isNarrow"
        :title="viewMode === 'single' ? t('reader.viewSpread') : t('reader.viewSingle')"
        @click="toggleViewMode"
      >
        <template #icon>
          <BlockOutlined v-if="viewMode === 'single'" />
          <FileTextOutlined v-else />
        </template>
      </a-button>

      <!-- Next Button -->
      <a-button
        type="text"
        size="small"
        class="flex h-auto items-center p-0 text-inherit opacity-60 transition-opacity hover:opacity-100"
        :title="t('reader.nextPage')"
        @click="emit('goForward')"
      >
        <template #icon><RightOutlined /></template>
      </a-button>
    </div>

    <!-- 2. Left Side Action Buttons -->
    <a-float-button
      type="primary"
      :style="{ left: '24px', bottom: '24px' }"
      class="soft-primary-btn"
      @click="closeReader"
    >
      <template #icon><HomeOutlined /></template>
      <template #tooltip>{{ t('reader.home') }}</template>
    </a-float-button>

    <a-float-button
      type="primary"
      :style="{ left: '24px', bottom: '80px' }"
      class="soft-primary-btn"
      @click="toggleSidebar"
    >
      <template #icon><UnorderedListOutlined /></template>
      <template #tooltip>{{ t('reader.toc') }}</template>
    </a-float-button>

    <!-- 3. Function Island (Bottom Right) -->
    <a-float-button-group
      trigger="click"
      type="primary"
      :style="{ right: '24px', bottom: '24px' }"
      class="soft-primary-btn"
    >
      <template #icon><AppstoreOutlined /></template>

      <a-float-button
        @click="toggleStudyPlan"
        :disabled="!canUseStudyPlan"
        :type="isStudyPlanActive ? 'primary' : 'default'"
        class="soft-primary-btn"
      >
        <template #icon>
          <CheckCircleOutlined v-if="isStudyPlanActive" />
          <CalendarOutlined v-else />
        </template>
        <template #tooltip>{{ studyPlanTooltip }}</template>
      </a-float-button>

      <!-- Hotspots Toggle -->
      <a-float-button @click="showHotspots = !showHotspots" type="primary" class="soft-primary-btn">
        <template #icon>
          <EyeOutlined v-if="showHotspots" />
          <EyeInvisibleOutlined v-else />
        </template>
        <template #tooltip>{{
          showHotspots ? t('reader.hideHotspots') : t('reader.showHotspots')
        }}</template>
      </a-float-button>

      <!-- Zoom Reset -->
      <a-float-button @click="readerStore.resetZoom()" type="primary" class="soft-primary-btn">
        <template #icon><FullscreenExitOutlined /></template>
        <template #tooltip>{{ t('reader.resetZoom') }}</template>
      </a-float-button>
    </a-float-button-group>

    <!-- Resources Drawer removed as per functionality cleanup -->
  </div>
</template>

<style scoped>
.reader-footer-floating {
  pointer-events: none;
}
.reader-footer-floating :deep(.ant-float-btn),
.reader-footer-floating :deep(.ant-btn),
.reader-footer-floating :deep(.ant-drawer) {
  pointer-events: auto;
}

.nav-btn {
  color: v-bind('token.colorTextSecondary');
}
.nav-btn:hover {
  color: v-bind('token.colorPrimary');
}

/* Soft Primary Button Styles */
:deep(.soft-primary-btn.ant-float-btn-primary .ant-float-btn-body) {
  background-color: v-bind('token.colorPrimaryBg') !important;
}
:deep(.soft-primary-btn.ant-float-btn-primary .ant-float-btn-icon) {
  color: v-bind('token.colorPrimary') !important;
  opacity: 0.65;
}
:deep(.soft-primary-btn.ant-float-btn-primary:hover .ant-float-btn-body) {
  background-color: v-bind('token.colorPrimaryBgHover') !important;
}
</style>
