import { mount, flushPromises } from '@vue/test-utils'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { createI18n } from 'vue-i18n'
import zh from '@/locales/zh.json'
import { getStudyAssessmentPreview } from '@/lib/api/studyPlan'
import { formatLocalDate } from '@/lib/datetime'
import Antd from 'ant-design-vue'
import StudyAssessmentModal from '../StudyAssessmentModal.vue'
import type { StudyTaskItem } from '../../../types'
import type { StudyAssessmentPreview } from '../../../types'

vi.mock('@/lib/api/studyPlan', () => ({ getStudyAssessmentPreview: vi.fn() }))

/** 构造后端预览响应，不在组件中重新实现算法。 */
function previewFixture(needsDecision = false): StudyAssessmentPreview {
  return {
    localDate: formatLocalDate(),
    revision: 'preview-version',
    sameDay: false,
    legacyDate: false,
    options: [
      {
        rating: 'forgotten',
        intervalDays: 1,
        scheduledDate: '2026-09-10',
        masteredStreak: 0,
        requiresFinishDecision: false,
      },
      {
        rating: 'hard',
        intervalDays: 3,
        scheduledDate: '2026-09-12',
        masteredStreak: 0,
        requiresFinishDecision: false,
      },
      {
        rating: 'good',
        intervalDays: 5,
        scheduledDate: '2026-09-14',
        masteredStreak: 0,
        requiresFinishDecision: false,
      },
      {
        rating: 'mastered',
        intervalDays: needsDecision ? 120 : 30,
        scheduledDate: '2026-10-09',
        masteredStreak: needsDecision ? 4 : 1,
        requiresFinishDecision: needsDecision,
      },
    ],
  }
}

beforeEach(() => {
  vi.mocked(getStudyAssessmentPreview).mockReset().mockResolvedValue(previewFixture())
})

/** 使用真实单选组件，替换弹窗容器以检查评估提交行为。 */
function mountAssessment(streak = 0) {
  if (streak >= 3) vi.mocked(getStudyAssessmentPreview).mockResolvedValue(previewFixture(true))
  const task: StudyTaskItem = {
    taskId: 5,
    planUnitId: 1,
    productCode: 'book',
    resourceId: 'unit',
    unitName: 'Unit 1',
    reviewStage: 5,
    scheduledDate: '2026-09-08',
    isOverdue: false,
    taskStatus: 0,
    completedAt: null,
    assessmentRequired: true,
    masteredStreak: streak,
  }
  return mount(StudyAssessmentModal, {
    props: { task, loading: false },
    global: {
      plugins: [Antd, createI18n({ legacy: false, locale: 'zh', messages: { zh } })],
      stubs: {
        AModal: {
          props: ['okButtonProps'],
          emits: ['ok', 'cancel'],
          template:
            '<div><header><slot name="title"/></header><main><slot/></main><button class="submit" :disabled="okButtonProps.disabled" @click="$emit(\'ok\')">提交</button><button class="cancel" @click="$emit(\'cancel\')">取消</button></div>',
        },
      },
    },
  })
}

describe('StudyAssessmentModal', () => {
  it('慢请求期间立即可选择和切换，返回后保留选择但此前禁止提交', async () => {
    let resolve!: (value: StudyAssessmentPreview) => void
    vi.mocked(getStudyAssessmentPreview).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done
      }),
    )
    const wrapper = mountAssessment()
    expect(wrapper.get('input[value="hard"]').attributes('disabled')).toBeUndefined()
    await wrapper.get('input[value="hard"]').setValue()
    await wrapper.get('input[value="good"]').setValue()
    expect(wrapper.get('.submit').attributes('disabled')).toBeDefined()
    expect(wrapper.find('.assessment-hints').exists()).toBe(false)
    resolve(previewFixture())
    await flushPromises()
    expect((wrapper.get('input[value="good"]').element as HTMLInputElement).checked).toBe(true)
    expect(wrapper.text()).toContain('5 天后')
    expect(wrapper.get('.submit').attributes('disabled')).toBeUndefined()
    await wrapper.setProps({ loading: true })
    expect(wrapper.get('input[value="good"]').attributes('disabled')).toBeDefined()
  })

  it('使用后端返回的动态间隔而非固定的熟练 14 天', async () => {
    const wrapper = mountAssessment()
    await flushPromises()
    await wrapper.get('input[value="good"]').setValue()
    expect(wrapper.text()).toContain('5 天后')
    expect(wrapper.text()).toContain('2026-09-14')
    await wrapper.get('.submit').trigger('click')
    expect(wrapper.emitted('submit')).toEqual([['good', undefined, 'preview-version']])
  })

  it('同日评价不根据旧计数要求结束，并显示限制说明', async () => {
    const result = previewFixture(false)
    result.sameDay = true
    vi.mocked(getStudyAssessmentPreview).mockResolvedValue(result)
    const wrapper = mountAssessment()
    await flushPromises()
    await wrapper.get('input[value="mastered"]').setValue()
    expect(wrapper.text()).toContain('同一天再次评估')
    expect(wrapper.find('.assessment-decision').exists()).toBe(false)
  })

  it('预览失败禁止提交，重试后恢复', async () => {
    vi.mocked(getStudyAssessmentPreview).mockRejectedValueOnce(new Error('offline'))
    const wrapper = mountAssessment()
    await flushPromises()
    expect(wrapper.text()).toContain('无法获取复习安排')
    expect(wrapper.get('.submit').attributes('disabled')).toBeDefined()
    const retry = wrapper.get('.assessment-retry')
    await wrapper.get('input[value="hard"]').setValue()
    await retry.trigger('click')
    await flushPromises()
    expect((wrapper.get('input[value="hard"]').element as HTMLInputElement).checked).toBe(true)
    expect(wrapper.get('.submit').attributes('disabled')).toBeUndefined()
  })

  it('跨日提交先刷新预览且要求重新选择', async () => {
    vi.mocked(getStudyAssessmentPreview).mockResolvedValueOnce({
      ...previewFixture(),
      localDate: '2000-01-01',
    })
    const wrapper = mountAssessment()
    await flushPromises()
    await wrapper.get('input[value="good"]').setValue()
    await wrapper.get('.submit').trigger('click')
    await flushPromises()
    expect(wrapper.emitted('submit')).toBeUndefined()
    expect(getStudyAssessmentPreview).toHaveBeenCalledTimes(2)
    expect(wrapper.text()).toContain('日期已变化')
    expect(wrapper.get('.submit').attributes('disabled')).toBeDefined()
  })

  it('关闭后忽略迟到的预览响应', async () => {
    let resolve!: (value: StudyAssessmentPreview) => void
    vi.mocked(getStudyAssessmentPreview).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done
      }),
    )
    const wrapper = mountAssessment()
    expect(wrapper.get('.submit').attributes('disabled')).toBeDefined()
    await wrapper.setProps({ task: null })
    resolve(previewFixture())
    await flushPromises()
    expect(wrapper.get('.submit').attributes('disabled')).toBeDefined()
    expect(wrapper.find('.assessment-hints').exists()).toBe(false)
  })

  it('单元名称仅在弹窗标题区显示', () => {
    const wrapper = mountAssessment()
    expect(wrapper.get('header').text()).toContain('掌握情况评估')
    expect(wrapper.get('header').text()).toContain('Unit 1')
    expect(wrapper.get('main').text()).not.toContain('Unit 1')
    wrapper.unmount()
  })

  it('历史任务的单元标题与目录编号格式一致', async () => {
    const wrapper = mountAssessment()
    await wrapper.setProps({
      task: { ...wrapper.props('task')!, resourceId: 'RE_00012', unitName: 'Describing character' },
    })
    expect(wrapper.get('header .assessment-unit').text()).toBe('Unit 12 Describing character')
    expect(wrapper.props('task')?.unitName).toBe('Describing character')
    wrapper.unmount()
  })

  it('必须选择评价，首次已掌握安排 30 天且不询问结束', async () => {
    const wrapper = mountAssessment()
    expect(wrapper.get('.submit').attributes('disabled')).toBeDefined()
    await flushPromises()
    await wrapper.get('input[value="mastered"]').setValue()
    expect(wrapper.text()).toContain('30 天后')
    expect(wrapper.text()).not.toContain('是否结束该学习计划')
    await wrapper.get('.submit').trigger('click')
    expect(wrapper.emitted('submit')).toEqual([['mastered', undefined, 'preview-version']])
    wrapper.unmount()
  })

  it.each([true, false])('第三次巩固要求明确选择结束或继续：%s', async (finish) => {
    const wrapper = mountAssessment(3)
    await flushPromises()
    await wrapper.get('input[value="mastered"]').setValue()
    expect(wrapper.text()).toContain('是否结束该学习计划')
    expect(wrapper.get('.submit').attributes('disabled')).toBeDefined()
    await wrapper.get(`input[value="${finish}"]`).setValue()
    await wrapper.get('.submit').trigger('click')
    expect(wrapper.emitted('submit')).toEqual([['mastered', finish, 'preview-version']])
    wrapper.unmount()
  })

  it('评价下降不要求结束选择；取消不提交完成', async () => {
    const wrapper = mountAssessment(4)
    await flushPromises()
    await wrapper.get('input[value="hard"]').setValue()
    expect(wrapper.text()).toContain('3 天后')
    expect(wrapper.text()).not.toContain('是否结束该学习计划')
    await wrapper.get('.cancel').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
    expect(wrapper.emitted('submit')).toBeUndefined()
    wrapper.unmount()
  })
})
