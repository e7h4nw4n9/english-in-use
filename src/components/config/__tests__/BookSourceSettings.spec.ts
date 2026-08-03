import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import BookSourceSettings from '../BookSourceSettings.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

const commonStubs = {
  'a-form': { template: '<form><slot /></form>' },
  'a-form-item': {
    props: ['label'],
    template: '<div><label>{{label}}</label><slot /></div>',
  },
  'a-radio-group': {
    props: ['value'],
    template: '<div class="radio-group-stub"><slot /></div>',
  },
  'a-radio-button': {
    props: ['value'],
    template:
      '<button class="radio-button-stub" @click="$parent.$emit(\'update:value\', value)"><slot /></button>',
  },
  'a-input': {
    props: ['value'],
    template:
      '<div><input class="input-stub" :value="value" @input="$emit(\'update:value\', $event.target.value)" /><slot name="addonAfter" /></div>',
  },
  'a-input-password': {
    props: ['value'],
    template:
      '<input type="password" class="input-password-stub" :value="value" @input="$emit(\'update:value\', $event.target.value)" />',
  },
  'a-button': {
    template: '<button class="button-stub" @click="$emit(\'click\')"><slot /></button>',
  },
  FolderOpenOutlined: { template: '<span class="folder-icon-stub" @click="$emit(\'click\')" />' },
}

describe('BookSourceSettings.vue', () => {
  const gatewayConfig = {
    base_url: 'https://gateway.example.com',
    access_token: 'token',
  }

  it('renders local folder settings when sourceType is Local', () => {
    const wrapper = mount(BookSourceSettings, {
      props: {
        sourceType: 'Local',
        localBookPath: '/test/path',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    const input = wrapper.find('input.input-stub')
    expect((input.element as HTMLInputElement).value).toBe('/test/path')
    expect(wrapper.text()).toContain('config.folderPath')
  })

  it('renders gateway settings when sourceType is CloudflareGateway', () => {
    const wrapper = mount(BookSourceSettings, {
      props: {
        sourceType: 'CloudflareGateway',
        localBookPath: '',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    expect(wrapper.text()).toContain('config.gatewayUrl')
    expect(wrapper.text()).toContain('config.gatewayToken')
    const inputs = wrapper.findAll('input')
    expect(inputs).toHaveLength(2)
  })

  it('emits update:sourceType when radio button is clicked', async () => {
    const wrapper = mount(BookSourceSettings, {
      props: {
        sourceType: 'Local',
        localBookPath: '',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    const gatewayButton = wrapper
      .findAll('.radio-button-stub')
      .find((b) => b.text().includes('config.cloudGateway'))
    await gatewayButton?.trigger('click')

    expect(wrapper.emitted()).toHaveProperty('update:sourceType')
    expect(wrapper.emitted()['update:sourceType'][0]).toEqual(['CloudflareGateway'])
  })

  it('emits select-folder when folder icon is clicked', async () => {
    const wrapper = mount(BookSourceSettings, {
      props: {
        sourceType: 'Local',
        localBookPath: '',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('.folder-icon-stub').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('select-folder')
  })
})
