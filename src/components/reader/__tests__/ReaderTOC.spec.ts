import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ReaderTOC from '../ReaderTOC.vue'
import { useReaderStore } from '../../../stores/reader'
import type { BookMetadata } from '../../../types'

// Partial mock for vue-i18n
vi.mock('vue-i18n', async (importOriginal) => {
  const actual = await importOriginal<typeof import('vue-i18n')>()
  return {
    ...actual,
    useI18n: () => ({
      t: (key: string) => key,
    }),
  }
})

// Partial mock for icons
vi.mock('@ant-design/icons-vue', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@ant-design/icons-vue')>()
  return {
    ...actual,
    // Add specific icons if they need to be mocked, otherwise let them be
  }
})

const mockMetadata: BookMetadata = {
  toc: [
    {
      title: 'Introductory Chapter',
      key: 'ch1',
      startPage: '1',
      endPage: '10',
      children: [
        { title: 'Section A', key: 's1.1', startPage: '1', endPage: '5' },
        { title: 'Section B', key: 's1.2', startPage: '6', endPage: '10' },
      ],
    },
    {
      title: 'Second Chapter',
      key: 'ch2',
      startPage: '11',
      endPage: '20',
    },
  ],
  pages: {},
  pageLabels: ['1', '2', '3', '4', '5', '6', '7', '8', '9', '10', '11'],
  pageWidth: 800,
  pageHeight: 1200,
}

describe('ReaderTOC', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    const readerStore = useReaderStore()
    readerStore.isSidebarCollapsed = false
  })

  it('renders TOC items correctly', () => {
    const wrapper = mount(ReaderTOC, {
      props: {
        metadata: mockMetadata,
      },
      global: {
        stubs: {
          'a-input': {
            template: '<input v-model="value" />',
            props: ['value'],
          },
          'a-collapse': {
            template: '<div class="a-collapse-stub"><slot /></div>',
          },
          'a-collapse-panel': {
            template:
              '<div class="a-collapse-panel-stub"><div class="panel-header"><slot name="header" /></div><div class="panel-content"><slot /></div></div>',
          },
        },
      },
    })

    expect(wrapper.text()).toContain('Introductory Chapter')
    expect(wrapper.text()).toContain('Second Chapter')
    expect(wrapper.text()).toContain('Section A')
  })

  it('parent nodes do not show page numbers', () => {
    const wrapper = mount(ReaderTOC, {
      props: {
        metadata: mockMetadata,
      },
      global: {
        stubs: {
          'a-input': true,
          'a-collapse': {
            template: '<div class="a-collapse-stub"><slot /></div>',
          },
          'a-collapse-panel': {
            template:
              '<div class="a-collapse-panel-stub"><div class="panel-header"><slot name="header" /></div><slot /></div>',
          },
        },
      },
    })

    const ch1Header = wrapper.find('.a-collapse-panel-stub .panel-header')
    expect(ch1Header.text()).toContain('Introductory Chapter')
    expect(ch1Header.text()).not.toContain('1') // Should not show page 1
  })

  it('clicking a parent node does not update current page', async () => {
    const wrapper = mount(ReaderTOC, {
      props: {
        metadata: mockMetadata,
      },
      global: {
        stubs: {
          'a-input': true,
          'a-collapse': {
            template: '<div class="a-collapse-stub"><slot /></div>',
          },
          'a-collapse-panel': {
            template:
              '<div class="a-collapse-panel-stub"><div class="panel-header" @click="$emit(\'click\')"><slot name="header" /></div><slot /></div>',
          },
        },
      },
    })

    const readerStore = useReaderStore()
    readerStore.currentPageLabel = '5'

    // Introductory Chapter is a parent node
    const ch1Header = wrapper.find('.a-collapse-panel-stub .panel-header')
    await ch1Header.trigger('click')

    // Page should still be 5, not updated to 1
    expect(readerStore.currentPageLabel).toBe('5')
  })

  it('updates current page when a leaf item is clicked', async () => {
    const wrapper = mount(ReaderTOC, {
      props: {
        metadata: mockMetadata,
      },
      global: {
        stubs: {
          'a-input': true,
          'a-collapse': {
            template: '<div class="a-collapse-stub"><slot /></div>',
          },
          'a-collapse-panel': {
            template: '<div class="a-collapse-panel-stub"><slot /></div>',
          },
        },
      },
    })

    const readerStore = useReaderStore()

    // Section A is a leaf node inside Introductory Chapter
    const s11 = wrapper.findAll('.toc-item').find((w) => w.text().includes('Section A'))
    await s11?.trigger('click')
    expect(readerStore.currentPageLabel).toBe('1')

    // Second Chapter is a top-level leaf node
    const ch2 = wrapper.findAll('.toc-item').find((w) => w.text().includes('Second Chapter'))
    await ch2?.trigger('click')
    expect(readerStore.currentPageLabel).toBe('11')
  })

  it('filters items when search text is entered', async () => {
    const wrapper = mount(ReaderTOC, {
      props: {
        metadata: mockMetadata,
      },
      global: {
        stubs: {
          'a-input': {
            template:
              '<input :value="value" @input="$emit(\'update:value\', $event.target.value)" />',
            props: ['value'],
          },
          'a-collapse': {
            template: '<div class="a-collapse-stub"><slot /></div>',
          },
          'a-collapse-panel': {
            template: '<div class="a-collapse-panel-stub"><slot /></div>',
          },
        },
      },
    })

    const input = wrapper.find('input')
    await input.setValue('Second Chapter')

    expect(wrapper.text()).toContain('Second Chapter')
    expect(wrapper.text()).not.toContain('Introductory Chapter')
  })

  it('toggles sidebar collapse state', async () => {
    const readerStore = useReaderStore()
    const wrapper = mount(ReaderTOC, {
      props: {
        metadata: mockMetadata,
      },
      global: {
        stubs: {
          'a-input': true,
        },
      },
    })

    // Close button is the second button in the header
    const closeBtn = wrapper.find('header button')
    await closeBtn.trigger('click')
    expect(readerStore.isSidebarCollapsed).toBe(true)
  })
})
