import { describe, expect, it, vi, beforeEach } from 'vitest'
import { ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { getExerciseHtml } from '@/lib/api/books'
import { extractExerciseRuntimePaths, prepareExerciseHtml } from '@/lib/exercise/runtime'
import { useReaderExerciseLoader } from '../useReaderExerciseLoader'
import type { Book } from '@/types'

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}))

vi.mock('@/lib/api/books', () => ({
  getExerciseHtml: vi.fn(),
}))

vi.mock('@/lib/exercise/runtime', () => ({
  prepareExerciseHtml: vi.fn((html: string) => ({ html: `prepared:${html}` })),
  extractExerciseRuntimePaths: vi.fn(() => ({
    engine: 'eiuasset://localhost/tmp/engine/',
    dp: 'eiuasset://localhost/tmp/dp/',
  })),
}))

function createAppStore() {
  const globalLoading = ref(false)
  const globalLoadingMessage = ref('')
  const globalLoadingProgress = ref<number | null>(null)

  return {
    globalLoading,
    globalLoadingMessage,
    globalLoadingProgress,
    startGlobalLoading: (message?: string) => {
      globalLoading.value = true
      globalLoadingMessage.value = message || ''
      globalLoadingProgress.value = null
    },
    setGlobalLoadingProgress: (progress: number | null) => {
      globalLoadingProgress.value = progress
    },
    setGlobalLoadingMessage: (message: string) => {
      globalLoadingMessage.value = message
    },
    stopGlobalLoading: () => {
      globalLoading.value = false
      globalLoadingMessage.value = ''
      globalLoadingProgress.value = null
    },
  }
}

describe('useReaderExerciseLoader', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('loads exercise html and updates reader state', async () => {
    const appStore = createAppStore()
    const unlisten = vi.fn()
    vi.mocked(listen).mockResolvedValue(unlisten)
    vi.mocked(getExerciseHtml).mockResolvedValue({
      html: '<html>exercise</html>',
      url: 'eiuasset://localhost/exercise.html',
    })

    const currentBook = ref({
      id: 1,
      book_group: 1,
      product_code: 'essgiuebk',
      title: 'Book',
      author: null,
      product_type: 'imgbook',
      cover: null,
      sort_num: 1,
    })
    const exerciseVisible = ref(false)
    const currentExerciseUrl = ref('stale-url')
    const currentExerciseHtml = ref('stale-html')
    const currentExerciseTitle = ref('')
    const currentExerciseResourceId = ref('stale-id')

    const { openExercise } = useReaderExerciseLoader({
      currentBook,
      appStore,
      exerciseVisible,
      currentExerciseUrl,
      currentExerciseHtml,
      currentExerciseTitle,
      currentExerciseResourceId,
      t: (key) => key,
    })

    await openExercise({ name: 'Practice 1', resource_id: 'RE_0001' })

    expect(getExerciseHtml).toHaveBeenCalledWith('essgiuebk', 'RE_0001')
    expect(prepareExerciseHtml).toHaveBeenCalledWith('<html>exercise</html>')
    expect(extractExerciseRuntimePaths).toHaveBeenCalledWith('prepared:<html>exercise</html>')
    expect(currentExerciseHtml.value).toBe('prepared:<html>exercise</html>')
    expect(currentExerciseTitle.value).toBe('Practice 1')
    expect(currentExerciseResourceId.value).toBe('RE_0001')
    expect(currentExerciseUrl.value).toBe('eiuasset://localhost/exercise.html')
    expect(exerciseVisible.value).toBe(true)
    expect(unlisten).toHaveBeenCalledTimes(1)
    expect(appStore.globalLoading.value).toBe(false)
  })

  it('updates global loading progress from download event', async () => {
    const appStore = createAppStore()
    let onProgress: (event: { payload: any }) => void = () => {}
    const unlisten = vi.fn()

    vi.mocked(listen).mockImplementation(async (_name, handler) => {
      onProgress = handler as (event: { payload: any }) => void
      return unlisten
    })

    let resolveHtml: (value: { html: string; url: string }) => void = () => {}
    vi.mocked(getExerciseHtml).mockReturnValue(
      new Promise((resolve) => {
        resolveHtml = resolve
      }),
    )

    const currentBook = ref({
      id: 1,
      book_group: 1,
      product_code: 'essgiuebk',
      title: 'Book',
      author: null,
      product_type: 'imgbook',
      cover: null,
      sort_num: 1,
    })
    const exerciseVisible = ref(false)
    const currentExerciseUrl = ref('')
    const currentExerciseHtml = ref('')
    const currentExerciseTitle = ref('')
    const currentExerciseResourceId = ref('')

    const { openExercise } = useReaderExerciseLoader({
      currentBook,
      appStore,
      exerciseVisible,
      currentExerciseUrl,
      currentExerciseHtml,
      currentExerciseTitle,
      currentExerciseResourceId,
      t: (key) => key,
    })

    const openPromise = openExercise({ name: 'Practice 2', resource_id: 'RE_0002' })

    expect(appStore.globalLoading.value).toBe(true)
    onProgress({
      payload: {
        productCode: 'essgiuebk',
        resourceId: 'RE_0002',
        stage: 'deps',
        totalFiles: 10,
        completedFiles: 4,
        failedFiles: 0,
        percent: 40,
        done: false,
      },
    })

    expect(appStore.globalLoadingProgress.value).toBe(40)
    expect(appStore.globalLoadingMessage.value).toBe('reader.loadingExerciseDeps (4/10)')

    resolveHtml({ html: '<html>ok</html>', url: 'eiuasset://localhost/ok.html' })
    await openPromise

    expect(unlisten).toHaveBeenCalledTimes(1)
    expect(appStore.globalLoading.value).toBe(false)
  })

  it('skips loading when no book selected or global loading is active', async () => {
    const appStore = createAppStore()
    vi.mocked(listen).mockResolvedValue(vi.fn())
    vi.mocked(getExerciseHtml).mockResolvedValue({
      html: '<html>noop</html>',
      url: 'eiuasset://localhost/noop.html',
    })

    const currentBook = ref<Book | null>(null)
    const exerciseVisible = ref(false)
    const currentExerciseUrl = ref('')
    const currentExerciseHtml = ref('')
    const currentExerciseTitle = ref('')
    const currentExerciseResourceId = ref('')

    const { openExercise } = useReaderExerciseLoader({
      currentBook,
      appStore,
      exerciseVisible,
      currentExerciseUrl,
      currentExerciseHtml,
      currentExerciseTitle,
      currentExerciseResourceId,
      t: (key) => key,
    })

    await openExercise({ name: 'Noop', resource_id: 'RE_X' })
    expect(getExerciseHtml).not.toHaveBeenCalled()

    currentBook.value = {
      id: 1,
      book_group: 1,
      product_code: 'essgiuebk',
      title: 'Book',
      author: null,
      product_type: 'imgbook',
      cover: null,
      sort_num: 1,
    }
    appStore.globalLoading.value = true

    await openExercise({ name: 'Busy', resource_id: 'RE_Y' })
    expect(getExerciseHtml).not.toHaveBeenCalled()
  })
})
