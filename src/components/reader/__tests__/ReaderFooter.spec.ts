import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ReaderFooter from '../ReaderFooter.vue'
import { useAppStore } from '../../../stores/app'
import { useReaderStore } from '../../../stores/reader'
import * as studyPlanApi from '../../../lib/api/studyPlan'
import type { StudyPlanStatusResponse } from '../../../types'

const modalConfirm = vi.hoisted(() => vi.fn())

vi.mock('vue-i18n', async (importOriginal) => {
  const actual = await importOriginal<typeof import('vue-i18n')>()
  return {
    ...actual,
    useI18n: () => ({
      t: (key: string) => key,
    }),
  }
})

vi.mock('ant-design-vue', () => ({
  Modal: {
    confirm: modalConfirm,
  },
  message: {
    success: vi.fn(),
    info: vi.fn(),
    error: vi.fn(),
  },
  theme: {
    useToken: () => ({
      token: {
        colorBgElevated: '#ffffff',
        colorBorderSecondary: '#d9d9d9',
        colorTextSecondary: '#666666',
        colorPrimary: '#1677ff',
        colorPrimaryBg: '#e6f4ff',
        colorPrimaryBgHover: '#bae0ff',
      },
    }),
  },
}))

vi.mock('../../../lib/api/studyPlan', () => ({
  getStudyPlanStatus: vi.fn(),
  upsertStudyPlan: vi.fn(),
  abandonStudyPlan: vi.fn(),
}))

function createStatus(overrides: Partial<StudyPlanStatusResponse> = {}): StudyPlanStatusResponse {
  return {
    inPlan: false,
    planStatus: null,
    planUnitId: null,
    completedStages: [],
    nextReviewDate: null,
    overdueCount: 0,
    ...overrides,
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

function mountFooter(props: Record<string, any> = {}) {
  return mount(ReaderFooter, {
    props: {
      displayIndex: 0,
      sortedPageLabels: ['1', '2', '3'],
      currentPageAudioFiles: [],
      currentStudyPlanResourceId: 'RE_SHARED',
      currentStudyPlanUnitName: 'Unit 1',
      timerStatus: 'idle',
      timerDisplay: '00:00:00',
      isNarrow: false,
      ...props,
    },
    global: {
      stubs: {
        'a-button': true,
        'a-float-button': true,
        'a-float-button-group': true,
        'a-popover': true,
        ReaderStudyTimerFloat: true,
        LeftOutlined: true,
        RightOutlined: true,
        UnorderedListOutlined: true,
        EyeOutlined: true,
        EyeInvisibleOutlined: true,
        FullscreenExitOutlined: true,
        FileTextOutlined: true,
        BlockOutlined: true,
        HomeOutlined: true,
        AppstoreOutlined: true,
        CalendarOutlined: true,
        CheckCircleOutlined: true,
        ClockCircleOutlined: true,
        PauseCircleOutlined: true,
        PlayCircleOutlined: true,
        RedoOutlined: true,
        SaveOutlined: true,
      },
    },
  })
}

describe('ReaderFooter study plan sync', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    setActivePinia(createPinia())
    vi.clearAllMocks()

    const appStore = useAppStore()
    const readerStore = useReaderStore()

    appStore.currentBook = {
      id: 1,
      book_group: 2,
      product_code: 'essgiuebk',
      title: 'Test Book',
      short_title: null,
      author: 'Author',
      product_type: 'imgbook',
      cover: null,
      sort_num: 1,
    }

    readerStore.currentPageLabel = '12'
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('resets status immediately when unit changes and reloads status', async () => {
    const mockedGetStudyPlanStatus = vi.mocked(studyPlanApi.getStudyPlanStatus)
    mockedGetStudyPlanStatus
      .mockResolvedValueOnce(createStatus({ inPlan: true, planStatus: 0, planUnitId: 101 }))
      .mockResolvedValueOnce(createStatus({ inPlan: false, planStatus: null, planUnitId: null }))

    const wrapper = mountFooter()
    const vm = wrapper.vm as unknown as {
      isStudyPlanActive: boolean
      studyPlanLoading: boolean
    }

    expect(vm.isStudyPlanActive).toBe(false)
    expect(vm.studyPlanLoading).toBe(true)

    vi.advanceTimersByTime(120)
    await flushPromises()
    expect(vm.isStudyPlanActive).toBe(true)
    expect(vm.studyPlanLoading).toBe(false)

    await wrapper.setProps({ currentStudyPlanUnitName: 'Unit 2' })
    expect(vm.isStudyPlanActive).toBe(false)
    expect(vm.studyPlanLoading).toBe(true)

    vi.advanceTimersByTime(120)
    await flushPromises()
    expect(vm.isStudyPlanActive).toBe(false)
    expect(vm.studyPlanLoading).toBe(false)
    expect(mockedGetStudyPlanStatus).toHaveBeenNthCalledWith(1, 'essgiuebk', 'RE_SHARED')
    expect(mockedGetStudyPlanStatus).toHaveBeenNthCalledWith(2, 'essgiuebk', 'RE_SHARED')
  })

  it('ignores stale status responses from previous unit context', async () => {
    const first = deferred<StudyPlanStatusResponse>()
    const second = deferred<StudyPlanStatusResponse>()
    const mockedGetStudyPlanStatus = vi.mocked(studyPlanApi.getStudyPlanStatus)
    mockedGetStudyPlanStatus
      .mockImplementationOnce(() => first.promise)
      .mockImplementationOnce(() => second.promise)

    const wrapper = mountFooter({
      currentStudyPlanResourceId: 'RE_1001',
      currentStudyPlanUnitName: 'Unit A',
    })
    const vm = wrapper.vm as unknown as {
      isStudyPlanActive: boolean
      studyPlanLoading: boolean
      studyPlanTooltip: string
    }

    vi.advanceTimersByTime(120)
    await flushPromises()
    expect(mockedGetStudyPlanStatus).toHaveBeenNthCalledWith(1, 'essgiuebk', 'RE_1001')

    await wrapper.setProps({
      currentStudyPlanResourceId: 'RE_2001',
      currentStudyPlanUnitName: 'Unit B',
    })
    expect(vm.studyPlanLoading).toBe(true)

    vi.advanceTimersByTime(120)
    await flushPromises()
    expect(mockedGetStudyPlanStatus).toHaveBeenNthCalledWith(2, 'essgiuebk', 'RE_2001')

    second.resolve(createStatus({ inPlan: false, planStatus: null, planUnitId: null }))
    await flushPromises()
    expect(vm.isStudyPlanActive).toBe(false)
    expect(vm.studyPlanLoading).toBe(false)
    expect(vm.studyPlanTooltip).toBe('studyPlan.add')

    first.resolve(createStatus({ inPlan: true, planStatus: 0, planUnitId: 501 }))
    await flushPromises()
    expect(vm.isStudyPlanActive).toBe(false)
    expect(vm.studyPlanTooltip).toBe('studyPlan.add')
  })

  it('clears active state when resource is unavailable', async () => {
    const mockedGetStudyPlanStatus = vi.mocked(studyPlanApi.getStudyPlanStatus)
    mockedGetStudyPlanStatus.mockResolvedValueOnce(
      createStatus({ inPlan: true, planStatus: 0, planUnitId: 303 }),
    )

    const wrapper = mountFooter({
      currentStudyPlanResourceId: 'RE_303',
      currentStudyPlanUnitName: 'Unit 303',
    })
    const vm = wrapper.vm as unknown as {
      isStudyPlanActive: boolean
      studyPlanTooltip: string
    }

    vi.advanceTimersByTime(120)
    await flushPromises()
    expect(vm.isStudyPlanActive).toBe(true)

    await wrapper.setProps({ currentStudyPlanResourceId: null })
    await flushPromises()
    expect(vm.isStudyPlanActive).toBe(false)
    expect(vm.studyPlanTooltip).toBe('studyPlan.unavailable')
    expect(mockedGetStudyPlanStatus).toHaveBeenCalledTimes(1)
  })

  it('keeps status stable when page changes within the same study plan unit', async () => {
    const mockedGetStudyPlanStatus = vi.mocked(studyPlanApi.getStudyPlanStatus)
    mockedGetStudyPlanStatus.mockResolvedValue(
      createStatus({ inPlan: true, planStatus: 0, planUnitId: 401 }),
    )

    const wrapper = mountFooter({
      currentStudyPlanResourceId: 'RE_401',
      currentStudyPlanUnitName: 'Unit 4',
      displayIndex: 0,
    })
    const vm = wrapper.vm as unknown as {
      isStudyPlanActive: boolean
      studyPlanLoading: boolean
    }

    vi.advanceTimersByTime(120)
    await flushPromises()
    expect(vm.isStudyPlanActive).toBe(true)
    expect(vm.studyPlanLoading).toBe(false)
    expect(mockedGetStudyPlanStatus).toHaveBeenCalledTimes(1)

    await wrapper.setProps({
      displayIndex: 1,
      currentPageAudioFiles: [{ path: 'exercise-audio.mp3' }],
    })
    await flushPromises()

    expect(vm.isStudyPlanActive).toBe(true)
    expect(vm.studyPlanLoading).toBe(false)
    expect(mockedGetStudyPlanStatus).toHaveBeenCalledTimes(1)
  })

  it('emits timerStart only from island when timer is idle', async () => {
    const wrapper = mountFooter({ timerStatus: 'idle' })
    const vm = wrapper.vm as unknown as {
      canStartTimerFromIsland: boolean
      startTimerFromIsland: () => void
    }

    expect(vm.canStartTimerFromIsland).toBe(true)
    vm.startTimerFromIsland()
    expect(wrapper.emitted('timerStart')).toHaveLength(1)

    await wrapper.setProps({ timerStatus: 'running' })
    expect(vm.canStartTimerFromIsland).toBe(false)
    vm.startTimerFromIsland()
    expect(wrapper.emitted('timerStart')).toHaveLength(1)
  })

  it('shows timer panel only when timer session is active', async () => {
    const wrapper = mountFooter({ timerStatus: 'idle' })
    const vm = wrapper.vm as unknown as {
      timerPanelVisible: boolean
    }

    expect(vm.timerPanelVisible).toBe(false)

    await wrapper.setProps({ timerStatus: 'running' })
    expect(vm.timerPanelVisible).toBe(true)

    await wrapper.setProps({ timerStatus: 'paused' })
    expect(vm.timerPanelVisible).toBe(true)

    await wrapper.setProps({ timerStatus: 'idle' })
    expect(vm.timerPanelVisible).toBe(false)
  })

  it('keeps the existing schedule and shows info when an active plan is added again', async () => {
    vi.mocked(studyPlanApi.getStudyPlanStatus).mockResolvedValue(createStatus())
    vi.mocked(studyPlanApi.upsertStudyPlan).mockResolvedValue({
      planUnitId: 501,
      planStatus: 0,
      nextReviewDate: '2026-08-11',
      totalStages: 7,
      completedStages: [],
      outcome: 'alreadyActive',
    })
    const { message } = await import('ant-design-vue')
    const appStore = useAppStore()
    const loadingAction = vi.spyOn(appStore, 'runGlobalLoadingAction')
    const wrapper = mountFooter()
    const vm = wrapper.vm as unknown as {
      toggleStudyPlan: () => Promise<void>
      studyPlanStatus: StudyPlanStatusResponse | null
    }

    vi.advanceTimersByTime(120)
    await flushPromises()
    await vm.toggleStudyPlan()
    await flushPromises()

    expect(studyPlanApi.upsertStudyPlan).toHaveBeenCalledOnce()
    expect(modalConfirm).not.toHaveBeenCalled()
    expect(loadingAction.mock.calls[loadingAction.mock.calls.length - 1]?.[1]).toBe(
      'studyPlan.adding',
    )
    expect(message.info).toHaveBeenCalledWith('studyPlan.alreadyActive')
    expect(message.success).not.toHaveBeenCalledWith('studyPlan.added')
    expect(vm.studyPlanStatus?.nextReviewDate).toBe('2026-08-11')
  })

  it('confirms before abandoning an active study plan', async () => {
    vi.mocked(studyPlanApi.getStudyPlanStatus).mockResolvedValue(
      createStatus({ inPlan: true, planStatus: 0, planUnitId: 601 }),
    )
    vi.mocked(studyPlanApi.abandonStudyPlan).mockResolvedValue({ success: true })
    const appStore = useAppStore()
    const loadingAction = vi.spyOn(appStore, 'runGlobalLoadingAction')
    const wrapper = mountFooter()
    const vm = wrapper.vm as unknown as {
      toggleStudyPlan: () => Promise<void>
    }

    vi.advanceTimersByTime(120)
    await flushPromises()
    const togglePromise = vm.toggleStudyPlan()
    await flushPromises()

    expect(modalConfirm).toHaveBeenCalledOnce()
    expect(studyPlanApi.abandonStudyPlan).not.toHaveBeenCalled()
    expect(modalConfirm.mock.calls[0]?.[0]).toMatchObject({
      title: 'studyPlan.cancelConfirmTitle',
      content: 'studyPlan.cancelConfirmDescription',
      okText: 'studyPlan.confirmCancel',
      cancelText: 'common.cancel',
      okButtonProps: { danger: true },
    })

    modalConfirm.mock.calls[0]?.[0].onOk()
    await togglePromise
    await flushPromises()

    expect(studyPlanApi.abandonStudyPlan).toHaveBeenCalledOnce()
    expect(loadingAction.mock.calls[loadingAction.mock.calls.length - 1]?.[1]).toBe(
      'studyPlan.abandoning',
    )
  })

  it('keeps the study plan when cancellation is dismissed', async () => {
    vi.mocked(studyPlanApi.getStudyPlanStatus).mockResolvedValue(
      createStatus({ inPlan: true, planStatus: 1, planUnitId: 701 }),
    )
    const appStore = useAppStore()
    const loadingAction = vi.spyOn(appStore, 'runGlobalLoadingAction')
    const wrapper = mountFooter()
    const vm = wrapper.vm as unknown as {
      toggleStudyPlan: () => Promise<void>
    }

    vi.advanceTimersByTime(120)
    await flushPromises()
    const togglePromise = vm.toggleStudyPlan()
    await flushPromises()

    expect(modalConfirm).toHaveBeenCalledOnce()
    modalConfirm.mock.calls[0]?.[0].onCancel()
    await togglePromise

    expect(studyPlanApi.abandonStudyPlan).not.toHaveBeenCalled()
    expect(loadingAction).not.toHaveBeenCalled()
  })
})
