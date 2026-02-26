import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import BookList from '../BookList.vue'
import * as api from '../../lib/api'
import { BookGroup } from '../../types'
import { createPinia, setActivePinia } from 'pinia'

// Mock the API
vi.mock('../../lib/api', () => ({
  getBooks: vi.fn(),
  getBookCover: vi.fn(),
  bytesToImageUrl: vi.fn(),
}))

// Mock vue-i18n
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
  createI18n: vi.fn(() => ({
    global: {
      t: (key: string) => key,
      locale: { value: 'en' },
    },
    install: vi.fn(),
  })),
}))

// Mock URL methods
global.URL.createObjectURL = vi.fn(() => 'mock-url')
global.URL.revokeObjectURL = vi.fn()

const commonStubs = {
  'a-spin': { template: '<div class="a-spin-stub"><slot /></div>' },
  'a-collapse': { template: '<div class="a-collapse-stub"><slot /></div>' },
  'a-collapse-panel': {
    props: ['header'],
    template:
      '<div class="a-collapse-panel-stub"><div class="panel-header"><slot name="header">{{header}}</slot></div><slot /></div>',
  },
  'a-tooltip': { template: '<div class="a-tooltip-stub"><slot /></div>' },
}

describe('BookList.vue', () => {
  const mockBooks = [
    {
      id: 1,
      book_group: BookGroup.Vocabulary,
      product_code: 'V1',
      title: 'Vocab 1',
      author: 'Author 1',
      product_type: 'Type 1',
      cover: 'cover1.jpg',
      sort_num: 1,
    },
    {
      id: 2,
      book_group: BookGroup.Grammar,
      product_code: 'G1',
      title: 'Grammar 1',
      author: 'Author 2',
      product_type: 'Type 2',
      cover: 'cover2.jpg',
      sort_num: 1,
    },
  ]

  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    ;(api.getBooks as any).mockResolvedValue(mockBooks)
    ;(api.getBookCover as any).mockResolvedValue(new Uint8Array([1, 2, 3]))
    ;(api.bytesToImageUrl as any).mockReturnValue('mock-url')
  })

  it('shows loading spinner initially', async () => {
    let resolveBooks: any
    ;(api.getBooks as any).mockReturnValue(
      new Promise((resolve) => {
        resolveBooks = resolve
      }),
    )

    const wrapper = mount(BookList, {
      global: {
        stubs: commonStubs,
      },
    })

    expect(wrapper.find('.a-spin-stub').exists()).toBe(true)

    resolveBooks(mockBooks)
    await flushPromises()
    expect(wrapper.find('.a-spin-stub').exists()).toBe(false)
  })

  it('shows empty state when no books are returned', async () => {
    ;(api.getBooks as any).mockResolvedValue([])

    const wrapper = mount(BookList, {
      global: {
        stubs: commonStubs,
      },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('暂无书籍数据')
  })

  it('renders books grouped by group', async () => {
    const wrapper = mount(BookList, {
      global: {
        stubs: commonStubs,
      },
    })

    await flushPromises()

    expect(wrapper.find('.panel-header').text()).toContain('app.bookGroups.vocabulary')
    expect(wrapper.text()).toContain('app.bookGroups.grammar')
    expect(wrapper.text()).toContain('Vocab 1')
    expect(wrapper.text()).toContain('Grammar 1')
  })

  it('calls revokeObjectURL on unmount', async () => {
    const wrapper = mount(BookList, {
      global: {
        stubs: commonStubs,
      },
    })
    await flushPromises()

    wrapper.unmount()
    expect(global.URL.revokeObjectURL).toHaveBeenCalledWith('mock-url')
  })

  it('sorts books by sort_num within groups', async () => {
    const unsortedBooks = [
      {
        id: 2,
        book_group: BookGroup.Vocabulary,
        title: 'B',
        sort_num: 2,
        cover: null,
        product_code: 'P2',
      },
      {
        id: 1,
        book_group: BookGroup.Vocabulary,
        title: 'A',
        sort_num: 1,
        cover: null,
        product_code: 'P1',
      },
    ]
    ;(api.getBooks as any).mockResolvedValue(unsortedBooks)

    const wrapper = mount(BookList, {
      global: {
        stubs: commonStubs,
      },
    })
    await flushPromises()

    const titles = wrapper.findAll('.book-title').map((w) => w.text())
    expect(titles).toEqual(['A', 'B'])
  })
})
