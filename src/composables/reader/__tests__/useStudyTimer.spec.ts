import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, ref, type Ref } from 'vue'
import { mount } from '@vue/test-utils'
import { saveStudySession } from '@/lib/api/studyTimer'
import { useStudyTimer } from '../useStudyTimer'

vi.mock('@/lib/api/studyTimer', () => ({
  saveStudySession: vi.fn(),
}))

interface TimerHarness {
  wrapper: ReturnType<typeof mount>
  api: ReturnType<typeof useStudyTimer>
  productCode: Ref<string | null>
  currentResourceId: Ref<string | null>
  currentUnitName: Ref<string>
  autoStart: Ref<boolean>
}

const saveStudySessionMock = vi.mocked(saveStudySession)

function setDocumentHidden(hidden: boolean) {
  Object.defineProperty(document, 'hidden', {
    configurable: true,
    get: () => hidden,
  })
}

function createHarness(options?: {
  productCode?: string | null
  currentResourceId?: string | null
  currentUnitName?: string
  autoStart?: boolean
}): TimerHarness {
  const productCode = ref<string | null>(options?.productCode ?? 'essgiuebk')
  const currentResourceId = ref<string | null>(options?.currentResourceId ?? 'RE_U1')
  const currentUnitName = ref<string>(options?.currentUnitName ?? 'Unit 1')
  const autoStart = ref<boolean>(options?.autoStart ?? false)

  let api: ReturnType<typeof useStudyTimer> | null = null
  const Harness = defineComponent({
    setup() {
      api = useStudyTimer({
        productCode,
        currentResourceId,
        currentUnitName,
        autoStart,
      })
      return () => h('div')
    },
  })

  const wrapper = mount(Harness)
  if (!api) {
    throw new Error('Study timer harness failed to initialize')
  }

  return {
    wrapper,
    api,
    productCode,
    currentResourceId,
    currentUnitName,
    autoStart,
  }
}

describe('useStudyTimer', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-03-01T00:00:00.000Z'))
    saveStudySessionMock.mockReset().mockResolvedValue({
      id: 1,
      success: true,
    })
    setDocumentHidden(false)
  })

  afterEach(() => {
    vi.useRealTimers()
    setDocumentHidden(false)
  })

  it('keeps same-resource page/exercise switch in one unit while tracking cross-unit visits', async () => {
    const harness = createHarness()

    expect(harness.api.start()).toBe(true)
    expect(harness.api.entryUnit.value?.resourceId).toBe('RE_U1')
    expect(harness.api.visitedUnits.value.map((item) => item.resourceId)).toEqual(['RE_U1'])

    harness.currentUnitName.value = 'Unit 1 Exercise'
    await nextTick()
    expect(harness.api.visitedUnits.value.map((item) => item.resourceId)).toEqual(['RE_U1'])

    harness.currentResourceId.value = 'RE_U2'
    harness.currentUnitName.value = 'Unit 2'
    await nextTick()

    vi.setSystemTime(new Date('2026-03-01T00:01:10.000Z'))
    const context = harness.api.buildStopContext()

    expect(context).not.toBeNull()
    expect(context?.entryUnit.resourceId).toBe('RE_U1')
    expect(context?.visitedUnits.map((item) => item.resourceId)).toEqual(['RE_U1', 'RE_U2'])

    harness.wrapper.unmount()
  })

  it('auto pauses when app is hidden', () => {
    const harness = createHarness()
    expect(harness.api.start()).toBe(true)
    expect(harness.api.status.value).toBe('running')

    setDocumentHidden(true)
    document.dispatchEvent(new Event('visibilitychange'))

    expect(harness.api.status.value).toBe('paused')
    expect(harness.api.autoPausedByBackground.value).toBe(true)

    harness.wrapper.unmount()
  })

  it('saves with assigned unit and resets timer state', async () => {
    const harness = createHarness()
    expect(harness.api.start()).toBe(true)

    harness.currentResourceId.value = 'RE_U2'
    harness.currentUnitName.value = 'Unit 2'
    await nextTick()
    vi.setSystemTime(new Date('2026-03-01T00:00:40.000Z'))

    const context = harness.api.buildStopContext()
    expect(context).not.toBeNull()
    if (!context) {
      harness.wrapper.unmount()
      return
    }

    await harness.api.saveWithAssignedUnit(context, {
      resourceId: 'RE_U2',
      unitName: 'Unit 2',
    })

    expect(saveStudySessionMock).toHaveBeenCalledTimes(1)
    const sessionEnd = new Date('2026-03-01T00:00:40.000Z')
    expect(saveStudySessionMock).toHaveBeenCalledWith({
      productCode: 'essgiuebk',
      entryResourceId: 'RE_U1',
      entryUnitName: 'Unit 1',
      assignedResourceId: 'RE_U2',
      assignedUnitName: 'Unit 2',
      visitedUnits: [
        {
          resourceId: 'RE_U1',
          unitName: 'Unit 1',
        },
        {
          resourceId: 'RE_U2',
          unitName: 'Unit 2',
        },
      ],
      startAt: '2026-03-01T00:00:00.000Z',
      endAt: '2026-03-01T00:00:40.000Z',
      duration: 40,
      localDate: `${sessionEnd.getFullYear()}-${String(sessionEnd.getMonth() + 1).padStart(2, '0')}-${String(sessionEnd.getDate()).padStart(2, '0')}`,
      timezoneOffsetMinutes: -sessionEnd.getTimezoneOffset(),
    })
    expect(harness.api.status.value).toBe('idle')
    expect(harness.api.visitedUnits.value).toEqual([])

    harness.wrapper.unmount()
  })

  it('auto starts when enabled', () => {
    const harness = createHarness({ autoStart: true })
    expect(harness.api.status.value).toBe('running')
    expect(harness.api.entryUnit.value?.resourceId).toBe('RE_U1')
    harness.wrapper.unmount()
  })
})
