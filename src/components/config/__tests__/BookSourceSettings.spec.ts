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
  const r2Config = {
    account_id: 'acc1',
    bucket_name: 'buck1',
    access_key_id: 'key1',
    secret_access_key: 'sec1',
    public_url: 'http://pub',
  }

  it('renders local folder settings when sourceType is Local', () => {
    const wrapper = mount(BookSourceSettings, {
      props: {
        sourceType: 'Local',
        localBookPath: '/test/path',
        r2Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    const input = wrapper.find('input.input-stub')
    expect((input.element as HTMLInputElement).value).toBe('/test/path')
    expect(wrapper.text()).toContain('config.folderPath')
  })

  it('renders R2 settings when sourceType is CloudflareR2', () => {
    const wrapper = mount(BookSourceSettings, {
      props: {
        sourceType: 'CloudflareR2',
        localBookPath: '',
        r2Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    expect(wrapper.text()).toContain('config.accountId')
    expect(wrapper.text()).toContain('config.bucketName')
    const inputs = wrapper.findAll('input')
    expect(inputs.length).toBeGreaterThan(3)
  })

  it('emits update:sourceType when radio button is clicked', async () => {
    const wrapper = mount(BookSourceSettings, {
      props: {
        sourceType: 'Local',
        localBookPath: '',
        r2Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    const r2Button = wrapper
      .findAll('.radio-button-stub')
      .find((b) => b.text().includes('config.cloudR2'))
    await r2Button?.trigger('click')

    expect(wrapper.emitted()).toHaveProperty('update:sourceType')
    expect(wrapper.emitted()['update:sourceType'][0]).toEqual(['CloudflareR2'])
  })

  it('emits select-folder when folder icon is clicked', async () => {
    const wrapper = mount(BookSourceSettings, {
      props: {
        sourceType: 'Local',
        localBookPath: '',
        r2Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('.folder-icon-stub').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('select-folder')
  })
})
