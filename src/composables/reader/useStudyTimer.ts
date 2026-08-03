import { computed, onBeforeUnmount, onMounted, ref, watch, type Ref } from 'vue'
import { saveStudySession } from '@/lib/api/studyTimer'
import type { SaveStudySessionPayload, StudySessionUnitRef } from '@/types'
import { formatLocalDate } from '@/lib/datetime'

export type StudyTimerStatus = 'idle' | 'running' | 'paused'

export interface StudyTimerStopContext {
  productCode: string
  entryUnit: StudySessionUnitRef
  visitedUnits: StudySessionUnitRef[]
  startAt: string
  endAt: string
  duration: number
}

interface UseStudyTimerOptions {
  productCode: Ref<string | null | undefined>
  currentResourceId: Ref<string | null>
  currentUnitName: Ref<string>
  autoStart: Ref<boolean>
}

/** 将当前资源信息规范化为可保存的计时单元。
 * @param resourceId - 当前资源标识。
 * @param unitName - 当前单元名称。
 */
function toTimerUnit(resourceId: string | null, unitName: string): StudySessionUnitRef | null {
  const normalizedResourceId = resourceId?.trim() || ''
  if (!normalizedResourceId) return null

  return {
    resourceId: normalizedResourceId,
    unitName: unitName.trim() || normalizedResourceId,
  }
}

function formatHms(totalSeconds: number): string {
  const safeTotal = Math.max(0, Math.floor(totalSeconds))
  const hours = Math.floor(safeTotal / 3600)
  const minutes = Math.floor((safeTotal % 3600) / 60)
  const seconds = safeTotal % 60

  return [hours, minutes, seconds].map((part) => String(part).padStart(2, '0')).join(':')
}

/**
 * 管理学习计时器生命周期、单元轨迹和会话保存。
 * @param options - 图书、当前单元和自动开始的响应式状态。
 */
export function useStudyTimer({
  productCode,
  currentResourceId,
  currentUnitName,
  autoStart,
}: UseStudyTimerOptions) {
  const status = ref<StudyTimerStatus>('idle')
  const sessionStartMs = ref<number | null>(null)
  const runningSinceMs = ref<number | null>(null)
  const elapsedFrozenMs = ref(0)
  const nowMs = ref(Date.now())
  const entryUnit = ref<StudySessionUnitRef | null>(null)
  const readerEntryUnit = ref<StudySessionUnitRef | null>(null)
  const trackedUnit = ref<StudySessionUnitRef | null>(null)
  const visitedUnits = ref<StudySessionUnitRef[]>([])
  const autoPausedByBackground = ref(false)
  const autoStartConsumed = ref(false)

  let nowTicker: ReturnType<typeof setInterval> | null = null

  const isRunning = computed(() => status.value === 'running')
  const hasActiveSession = computed(() => status.value !== 'idle')

  const elapsedMs = computed(() => {
    if (!hasActiveSession.value) return 0
    if (status.value !== 'running' || runningSinceMs.value === null) {
      return elapsedFrozenMs.value
    }
    return elapsedFrozenMs.value + Math.max(0, nowMs.value - runningSinceMs.value)
  })

  const elapsedSeconds = computed(() => Math.max(0, Math.floor(elapsedMs.value / 1000)))
  const formattedDuration = computed(() => formatHms(elapsedSeconds.value))
  const displayUnitName = computed(
    () =>
      trackedUnit.value?.unitName ||
      entryUnit.value?.unitName ||
      currentUnitName.value.trim() ||
      currentResourceId.value ||
      '',
  )

  function updateNow() {
    nowMs.value = Date.now()
  }

  function stopTicker() {
    if (!nowTicker) return
    clearInterval(nowTicker)
    nowTicker = null
  }

  function startTicker() {
    stopTicker()
    nowTicker = setInterval(() => {
      updateNow()
    }, 500)
  }

  function resolveCurrentUnit(): StudySessionUnitRef | null {
    return toTimerUnit(currentResourceId.value, currentUnitName.value)
  }

  function pushVisitedUnit(unit: StudySessionUnitRef) {
    if (!visitedUnits.value.some((item) => item.resourceId === unit.resourceId)) {
      visitedUnits.value = [...visitedUnits.value, unit]
    }
    trackedUnit.value = unit
  }

  /** 从阅读器入口单元开始一次新的计时会话。 */
  function start(): boolean {
    if (status.value !== 'idle') return false
    const code = productCode.value?.trim() || ''
    if (!code) return false

    const currentUnit = resolveCurrentUnit()
    const unit = readerEntryUnit.value || currentUnit
    if (!unit) return false

    const startMs = Date.now()
    sessionStartMs.value = startMs
    runningSinceMs.value = startMs
    elapsedFrozenMs.value = 0
    entryUnit.value = unit
    trackedUnit.value = currentUnit || unit
    visitedUnits.value = [unit]
    if (currentUnit && currentUnit.resourceId !== unit.resourceId) {
      visitedUnits.value = [unit, currentUnit]
    }
    autoPausedByBackground.value = false
    status.value = 'running'
    autoStartConsumed.value = true
    updateNow()
    return true
  }

  /** 冻结当前已用时间并进入暂停状态。 */
  function pause(): boolean {
    if (status.value !== 'running' || runningSinceMs.value === null) return false

    const pauseAt = Date.now()
    elapsedFrozenMs.value += Math.max(0, pauseAt - runningSinceMs.value)
    runningSinceMs.value = null
    status.value = 'paused'
    updateNow()
    return true
  }

  /** 从暂停状态继续累计当前会话时间。 */
  function resume(): boolean {
    if (status.value !== 'paused') return false
    runningSinceMs.value = Date.now()
    status.value = 'running'
    autoPausedByBackground.value = false
    updateNow()
    return true
  }

  /** 清空当前会话状态。
   * @param keepAutoStartConsumed - 是否保留自动开始已消费标记。
   */
  function reset(keepAutoStartConsumed = true) {
    status.value = 'idle'
    sessionStartMs.value = null
    runningSinceMs.value = null
    elapsedFrozenMs.value = 0
    entryUnit.value = null
    trackedUnit.value = null
    visitedUnits.value = []
    autoPausedByBackground.value = false
    if (!keepAutoStartConsumed) {
      autoStartConsumed.value = false
      readerEntryUnit.value = null
    }
    updateNow()
  }

  function restart(): boolean {
    reset(true)
    return start()
  }

  /** 生成停止确认和持久化共用的不可变会话快照。 */
  function buildStopContext(): StudyTimerStopContext | null {
    if (!hasActiveSession.value) return null

    const code = productCode.value?.trim() || ''
    const startAtMs = sessionStartMs.value
    const entry = entryUnit.value
    if (!code || !entry || startAtMs === null) {
      return null
    }

    const endAtMs = Date.now()
    const runningPartMs =
      status.value === 'running' && runningSinceMs.value !== null
        ? Math.max(0, endAtMs - runningSinceMs.value)
        : 0
    const totalMs = Math.max(0, elapsedFrozenMs.value + runningPartMs)
    const duration = Math.floor(totalMs / 1000)
    if (duration <= 0) return null

    return {
      productCode: code,
      entryUnit: entry,
      visitedUnits: visitedUnits.value.length ? [...visitedUnits.value] : [entry],
      startAt: new Date(startAtMs).toISOString(),
      endAt: new Date(endAtMs).toISOString(),
      duration,
    }
  }

  /** 将停止快照保存到用户确认的归属单元。
   * @param context - 停止时捕获的会话快照。
   * @param assignedUnit - 会话最终归属单元。
   */
  async function saveWithAssignedUnit(
    context: StudyTimerStopContext,
    assignedUnit: StudySessionUnitRef,
  ): Promise<void> {
    const payload: SaveStudySessionPayload = {
      productCode: context.productCode,
      entryResourceId: context.entryUnit.resourceId,
      entryUnitName: context.entryUnit.unitName,
      assignedResourceId: assignedUnit.resourceId,
      assignedUnitName: assignedUnit.unitName,
      visitedUnits: context.visitedUnits,
      startAt: context.startAt,
      endAt: context.endAt,
      duration: context.duration,
      localDate: formatLocalDate(new Date(context.endAt)),
      timezoneOffsetMinutes: -new Date(context.endAt).getTimezoneOffset(),
    }
    await saveStudySession(payload)
    reset(true)
  }

  function markDiscarded() {
    reset(true)
  }

  /** 页面进入后台时自动暂停，返回前台时恢复自动暂停的会话。 */
  function handleVisibilityChange() {
    if (!document.hidden) return
    if (status.value !== 'running') return
    const paused = pause()
    if (paused) {
      autoPausedByBackground.value = true
    }
  }

  watch(isRunning, (running) => {
    if (running) {
      startTicker()
      return
    }
    stopTicker()
  })

  watch(
    [currentResourceId, currentUnitName],
    () => {
      const unit = resolveCurrentUnit()
      if (!unit) return
      if (!readerEntryUnit.value) {
        readerEntryUnit.value = unit
      }
      if (hasActiveSession.value) {
        pushVisitedUnit(unit)
      }
    },
    { immediate: true },
  )

  watch(
    [productCode, currentResourceId, autoStart],
    () => {
      if (!autoStart.value || autoStartConsumed.value) return
      if (status.value !== 'idle') return
      if (!productCode.value || !currentResourceId.value) return
      start()
    },
    { immediate: true },
  )

  watch(
    productCode,
    (next, prev) => {
      if (next === prev) return
      reset(false)
    },
    { flush: 'sync' },
  )

  onMounted(() => {
    document.addEventListener('visibilitychange', handleVisibilityChange)
    updateNow()
  })

  onBeforeUnmount(() => {
    document.removeEventListener('visibilitychange', handleVisibilityChange)
    stopTicker()
  })

  return {
    status,
    hasActiveSession,
    isRunning,
    elapsedSeconds,
    formattedDuration,
    displayUnitName,
    entryUnit,
    visitedUnits,
    autoPausedByBackground,
    start,
    pause,
    resume,
    restart,
    reset,
    buildStopContext,
    saveWithAssignedUnit,
    markDiscarded,
  }
}
