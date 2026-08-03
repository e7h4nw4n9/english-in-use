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
    props: ['value', 'placeholder'],
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
  FolderOpenOutlined: {
    template: '<span class="choose-path-icon-stub" @click="$emit(\'click\')" />',
  },
  ReloadOutlined: {
    template: '<span class="restore-default-icon-stub" @click="$emit(\'click\')" />',
  },
}

describe('DatabaseSettings.vue', () => {
  const gatewayConfig = {
    base_url: 'https://gateway.example.com',
    access_token: 'token',
  }

  it('renders SQLite settings when dbType is SQLite', () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '/test/db.sqlite',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    const input = wrapper.find('input.input-stub')
    expect((input.element as HTMLInputElement).value).toBe('/test/db.sqlite')
    expect(wrapper.text()).toContain('config.filePath')
  })

  it('renders gateway settings when dbType is CloudflareGateway', () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'CloudflareGateway',
        sqlitePath: '',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    expect(wrapper.text()).toContain('config.gatewayUrl')
    expect(wrapper.text()).toContain('config.gatewayToken')
  })

  it('emits update:dbType when radio button is clicked', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    const gatewayButton = wrapper
      .findAll('.radio-button-stub')
      .find((b) => b.text().includes('config.cloudGateway'))
    await gatewayButton?.trigger('click')

    expect(wrapper.emitted()).toHaveProperty('update:dbType')
    expect(wrapper.emitted()['update:dbType'][0]).toEqual(['CloudflareGateway'])
  })

  it('emits copy-path when copy icon is clicked', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '/test/path',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('.copy-icon-stub').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('copy-path')
    expect(wrapper.emitted()['copy-path'][0]).toEqual(['/test/path'])
  })

  it('emits update:sqlitePath when sqlite path is edited', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '/test/path.db',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('input.input-stub').setValue('/custom/new-path')
    expect(wrapper.emitted()).toHaveProperty('update:sqlitePath')
    expect(wrapper.emitted()['update:sqlitePath'][0]).toEqual(['/custom/new-path'])
  })

  it('emits choose-sqlite-path when choose icon is clicked', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '/test/path.db',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('.choose-path-icon-stub').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('choose-sqlite-path')
  })

  it('emits restore-default-sqlite-path when restore icon is clicked', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '/test/path.db',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('.restore-default-icon-stub').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('restore-default-sqlite-path')
  })

  it('emits test-connection when test button is clicked', async () => {
    const wrapper = mount(DatabaseSettings, {
      props: {
        dbType: 'SQLite',
        sqlitePath: '',
        gatewayConfig,
        isTesting: false,
      },
      global: { stubs: commonStubs },
    })

    await wrapper.find('.button-stub').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('test-connection')
  })
})
