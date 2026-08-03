import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { defineComponent, ref } from 'vue'
import type { Book, BookMetadata, ReadingProgress } from '@/types'
import { useReaderProgress } from '../useReaderProgress'

const booksApi = vi.hoisted(() => ({
  getReadingProgress: vi.fn(),
  updateReadingProgress: vi.fn(),
}))

vi.mock('@/lib/api/books', () => booksApi)

const book = {
  product_code: 'book-1',
} as Book

const metadata: BookMetadata = {
  toc: [],
  pages: {
    '1': { label: '1', image_path: '1.jpg', resource_id: 'resource-1' },
    '2': { label: '2', image_path: '2.jpg', resource_id: 'resource-2' },
    '3': { label: '3', image_path: '3.jpg', resource_id: 'resource-3' },
  },
  pageLabels: ['1', '2', '3'],
  pageWidth: 100,
  pageHeight: 100,
}

describe('useReaderProgress', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('does not save a temporary page while restoring progress', async () => {
    let resolveProgress!: (progress: ReadingProgress) => void
    booksApi.getReadingProgress.mockReturnValue(
      new Promise<ReadingProgress>((resolve) => {
        resolveProgress = resolve
      }),
    )
    booksApi.updateReadingProgress.mockResolvedValue(undefined)
    const currentPageLabel = ref('1')
    let progress!: ReturnType<typeof useReaderProgress>
    const wrapper = mount(
      defineComponent({
        setup() {
          progress = useReaderProgress({
            currentBook: ref(book),
            metadata: ref(metadata),
            currentPageLabel,
            zoomLevel: ref(1),
            sortedPageLabels: ref(metadata.pageLabels),
          })
          return () => null
        },
      }),
    )

    const restoring = progress.restoreProgress()
    currentPageLabel.value = '2'
    progress.saveProgress()
    await progress.flushProgress()
    expect(booksApi.updateReadingProgress).not.toHaveBeenCalled()

    resolveProgress({
      book_id: 1,
      resource_id: 'resource-2',
      page_label: '2',
      scale: 1,
      offset_x: 0,
      offset_y: 0,
      updated_at: '',
    })
    await restoring
    currentPageLabel.value = '3'
    progress.saveProgress()
    await progress.flushProgress()

    expect(booksApi.updateReadingProgress).toHaveBeenCalledOnce()
    expect(booksApi.updateReadingProgress).toHaveBeenCalledWith(
      'book-1',
      'resource-3',
      '3',
      1,
      0,
      0,
    )
    wrapper.unmount()
  })

  it('serializes writes so an older request cannot overwrite the latest page', async () => {
    let resolveFirstSave!: () => void
    booksApi.updateReadingProgress
      .mockReturnValueOnce(
        new Promise<void>((resolve) => {
          resolveFirstSave = resolve
        }),
      )
      .mockResolvedValueOnce(undefined)
    const currentPageLabel = ref('1')
    let progress!: ReturnType<typeof useReaderProgress>
    const wrapper = mount(
      defineComponent({
        setup() {
          progress = useReaderProgress({
            currentBook: ref(book),
            metadata: ref(metadata),
            currentPageLabel,
            zoomLevel: ref(1),
            sortedPageLabels: ref(metadata.pageLabels),
          })
          return () => null
        },
      }),
    )

    progress.saveProgress()
    const firstFlush = progress.flushProgress()
    currentPageLabel.value = '2'
    progress.saveProgress()
    const secondFlush = progress.flushProgress()
    expect(booksApi.updateReadingProgress).toHaveBeenCalledOnce()

    resolveFirstSave()
    await Promise.all([firstFlush, secondFlush])

    expect(booksApi.updateReadingProgress).toHaveBeenCalledTimes(2)
    expect(booksApi.updateReadingProgress.mock.calls[1]?.[2]).toBe('2')
    wrapper.unmount()
  })

  it('skips a snapshot that was already persisted successfully', async () => {
    booksApi.updateReadingProgress.mockResolvedValue(undefined)
    let progress!: ReturnType<typeof useReaderProgress>
    const wrapper = mount(
      defineComponent({
        setup() {
          progress = useReaderProgress({
            currentBook: ref(book),
            metadata: ref(metadata),
            currentPageLabel: ref('1'),
            zoomLevel: ref(1),
            sortedPageLabels: ref(metadata.pageLabels),
          })
          return () => null
        },
      }),
    )

    progress.saveProgress()
    await progress.flushProgress()
    progress.saveProgress()
    await progress.flushProgress()

    expect(booksApi.updateReadingProgress).toHaveBeenCalledOnce()
    wrapper.unmount()
  })
})
