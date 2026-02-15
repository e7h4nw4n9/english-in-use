import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import SystemSettings from '../SystemSettings.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('../../lib/api/database', () => ({
  getMigrationVersions: vi.fn(() => Promise.resolve(['0.1.0'])),
  getCurrentDbVersion: vi.fn(() => Promise.resolve('0.1.0')),
  executeMigrationUp: vi.fn(),
  executeMigrationDown: vi.fn(),
}))

const commonStubs = {
  'a-form': { template: '<form><slot /></form>' },
  'a-form-item': {
    props: ['label'],
    template: '<div><label>{{label}}</label><slot /></div>',
  },
  'a-select': {
    props: ['value'],
    template:
      '<select class="select-stub" :value="value" @change="$emit(\'update:value\', $event.target.value)"><slot /></select>',
  },
  'a-select-option': {
    props: ['value'],
    template: '<option :value="value"><slot /></option>',
  },
  'a-switch': {
    props: ['checked', 'disabled'],
    template:
      '<input type="checkbox" class="switch-stub" :checked="checked" :disabled="disabled" @change="$emit(\'update:checked\', $event.target.checked)" />',
  },
  'a-input-number': {
    props: ['value'],
    template:
      '<input type="number" class="input-number-stub" :value="value" @input="$emit(\'update:value\', Number($event.target.value))" />',
  },
  'a-tooltip': { template: '<div><slot /></div>' },
  'a-divider': { template: '<hr />' },
  'a-tag': { template: '<span><slot /></span>' },
  'a-space': { template: '<div><slot /></div>' },
  'a-button': { template: '<button @click="$emit(\'click\')"><slot /></button>' },
}

describe('SystemSettings.vue', () => {
  it('renders all form items correctly', () => {
    const wrapper = mount(SystemSettings, {
      props: {
        language: 'en',
        themeMode: 'system',
        logLevel: 'info',
        enableAutoCheck: true,
        checkIntervalMins: 5,
        isCloudConfigured: true,
      },
      global: { stubs: commonStubs },
    })

    expect(wrapper.text()).toContain('config.language')
    expect(wrapper.text()).toContain('config.theme')
    expect(wrapper.text()).toContain('config.logLevel')
    expect(wrapper.text()).toContain('config.enableAutoCheck')
    expect(wrapper.text()).toContain('config.checkInterval')
  })

  it('disables auto check switch when cloud is not configured', () => {
    const wrapper = mount(SystemSettings, {
      props: {
        language: 'en',
        themeMode: 'system',
        logLevel: 'info',
        enableAutoCheck: true,
        checkIntervalMins: 5,
        isCloudConfigured: false,
      },
      global: { stubs: commonStubs },
    })

    const switchInput = wrapper.find('.switch-stub')
    expect((switchInput.element as HTMLInputElement).disabled).toBe(true)
  })

  it('emits updates when values change', async () => {
    const wrapper = mount(SystemSettings, {
      props: {
        language: 'en',
        themeMode: 'system',
        logLevel: 'info',
        enableAutoCheck: true,
        checkIntervalMins: 5,
        isCloudConfigured: true,
      },
      global: { stubs: commonStubs },
    })

    const langSelect = wrapper.find('.select-stub')
    await langSelect.setValue('zh')
    expect(wrapper.emitted()).toHaveProperty('update:language')
    expect(wrapper.emitted()['update:language'][0]).toEqual(['zh'])
  })
})
