import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { describe, it, expect } from 'vitest'
import StudyPlanRules from '../StudyPlanRules.vue'
import zh from '@/locales/zh.json'
import en from '@/locales/en.json'

describe('学习计划规则说明', () => {
  it.each(['zh', 'en'])('显示完整的只读说明：%s', (locale) => {
    const wrapper = mount(StudyPlanRules, {
      global: { plugins: [createI18n({ legacy: false, locale, messages: { zh, en } })] },
    })
    expect(wrapper.findAll('section')).toHaveLength(5)
    expect(wrapper.findAll('tbody tr')).toHaveLength(4)
    expect(wrapper.text()).toContain('I × 0.5')
    expect(wrapper.text()).toContain('I × 1.5')
    expect(wrapper.text()).toContain('3 → 5 → 8 → 12 → 18 → 27 → 30')
    expect(wrapper.text()).not.toContain('studyPlanRules.')
    expect(wrapper.find('input, select, button, form').exists()).toBe(false)
  })
})
