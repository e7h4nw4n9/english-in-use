import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import DatabaseSettings from '../DatabaseSettings.vue'

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
  'a-tooltip': {
    props: ['title'],
    template: '<div class="tooltip-stub" :title="title"><slot /></div>',
  },
  CopyOutlined: { template: '<span class="copy-icon-stub" @click="$emit(\'click\')" />' },
}

describe('DatabaseSettings.vue', () => {
  const d1Config = {
    account_id: 'acc1',
    database_id: 'db1',
    api_token: 'tok1',
  }

  it('renders SQLite settings when dbType is SQLite', () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '/test/db.sqlite',
        d1Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    const input = wrapper.find('input.input-stub')
    expect((input.element as HTMLInputElement).value).toBe('/test/db.sqlite')
    expect(wrapper.text()).toContain('config.filePath')
  })

  it('renders D1 settings when dbType is CloudflareD1', () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'CloudflareD1',
        sqlitePath: '',
        d1Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    expect(wrapper.text()).toContain('config.accountId')
    expect(wrapper.text()).toContain('config.databaseId')
    expect(wrapper.text()).toContain('config.apiToken')
  })

  it('emits update:dbType when radio button is clicked', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '',
        d1Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    const d1Button = wrapper
      .findAll('.radio-button-stub')
      .find((b) => b.text().includes('config.cloudD1'))
    await d1Button?.trigger('click')

    expect(wrapper.emitted()).toHaveProperty('update:dbType')
    expect(wrapper.emitted()['update:dbType'][0]).toEqual(['CloudflareD1'])
  })

  it('emits copy-path when copy icon is clicked', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '/test/path',
        d1Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('.copy-icon-stub').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('copy-path')
    expect(wrapper.emitted()['copy-path'][0]).toEqual(['/test/path'])
  })

  it('emits test-connection when test button is clicked', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '',
        d1Config,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('.button-stub').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('test-connection')
  })
})
