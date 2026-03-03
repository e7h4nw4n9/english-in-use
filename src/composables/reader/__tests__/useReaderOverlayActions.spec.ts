import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { useReaderOverlayActions } from '../useReaderOverlayActions'
import type { Book, OverlayItem } from '@/types'

function createBook(): Book {
  return {
    id: 1,
    book_group: 1,
    product_code: 'essgiuebk',
    title: 'Test Book',
    author: null,
    product_type: 'imgbook',
    cover: null,
    sort_num: 1,
  }
}

describe('useReaderOverlayActions', () => {
  it('updates current page when page overlay is clicked', async () => {
    const currentPageLabel = ref('12')
    const openExercise = vi.fn()
    const toggleAudio = vi.fn()

    const { handleOverlayClick } = useReaderOverlayActions({
      currentBook: ref(createBook()),
      currentPageLabel,
      openExercise,
      toggleAudio,
    })

    const overlay: OverlayItem = {
      x: 0,
      y: 0,
      w: 10,
      h: 10,
      type: 'page',
      page: { pagelabel: '15' },
    }

    await handleOverlayClick(overlay)

    expect(currentPageLabel.value).toBe('15')
    expect(toggleAudio).not.toHaveBeenCalled()
    expect(openExercise).not.toHaveBeenCalled()
  })

  it('toggles audio when audio overlay is clicked with current book', async () => {
    const currentPageLabel = ref('12')
    const openExercise = vi.fn()
    const toggleAudio = vi.fn()

    const { handleOverlayClick } = useReaderOverlayActions({
      currentBook: ref(createBook()),
      currentPageLabel,
      openExercise,
      toggleAudio,
    })

    const overlay: OverlayItem = {
      x: 0,
      y: 0,
      w: 10,
      h: 10,
      type: 'audio',
      audio: { path: 'overlays/audio/unit1.mp3' },
    }

    await handleOverlayClick(overlay)

    expect(toggleAudio).toHaveBeenCalledWith('essgiuebk', 'overlays/audio/unit1.mp3')
    expect(openExercise).not.toHaveBeenCalled()
  })

  it('opens exercise for exercise and learning-object overlays', async () => {
    const currentPageLabel = ref('12')
    const openExercise = vi.fn()
    const toggleAudio = vi.fn()

    const { handleOverlayClick } = useReaderOverlayActions({
      currentBook: ref(createBook()),
      currentPageLabel,
      openExercise,
      toggleAudio,
    })

    const exerciseOverlay: OverlayItem = {
      x: 0,
      y: 0,
      w: 10,
      h: 10,
      type: 'exercise',
      exercise: { name: 'Practice 1', resource_id: 'RE_001' },
    }

    const learningObjectOverlay: OverlayItem = {
      x: 0,
      y: 0,
      w: 10,
      h: 10,
      type: 'learning-object',
      exercise: { name: 'Learning Obj', resource_id: 'RE_002' },
    }

    await handleOverlayClick(exerciseOverlay)
    await handleOverlayClick(learningObjectOverlay)

    expect(openExercise).toHaveBeenNthCalledWith(1, { name: 'Practice 1', resource_id: 'RE_001' })
    expect(openExercise).toHaveBeenNthCalledWith(2, {
      name: 'Learning Obj',
      resource_id: 'RE_002',
    })
  })

  it('does nothing for audio overlay when no current book', async () => {
    const currentPageLabel = ref('12')
    const openExercise = vi.fn()
    const toggleAudio = vi.fn()

    const { handleOverlayClick } = useReaderOverlayActions({
      currentBook: ref<Book | null>(null),
      currentPageLabel,
      openExercise,
      toggleAudio,
    })

    await handleOverlayClick({
      x: 0,
      y: 0,
      w: 10,
      h: 10,
      type: 'audio',
      audio: { path: 'overlays/audio/unit1.mp3' },
    })

    expect(toggleAudio).not.toHaveBeenCalled()
    expect(openExercise).not.toHaveBeenCalled()
  })
})
