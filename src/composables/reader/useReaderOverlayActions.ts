import type { Ref } from 'vue'
import type { Book, ExerciseInfo, OverlayItem } from '@/types'

interface UseReaderOverlayActionsOptions {
  currentBook: Ref<Book | null>
  currentPageLabel: Ref<string>
  openExercise: (exercise: ExerciseInfo) => void | Promise<void>
  toggleAudio: (productCode: string, path: string) => void | Promise<void>
}

export function useReaderOverlayActions({
  currentBook,
  currentPageLabel,
  openExercise,
  toggleAudio,
}: UseReaderOverlayActionsOptions) {
  async function handleOverlayClick(overlay: OverlayItem) {
    if (overlay.type === 'page' && overlay.page) {
      currentPageLabel.value = overlay.page.pagelabel
      return
    }

    if (overlay.type === 'audio' && overlay.audio) {
      if (!currentBook.value) return
      await toggleAudio(currentBook.value.product_code, overlay.audio.path)
      return
    }

    if ((overlay.type === 'exercise' || overlay.type === 'learning-object') && overlay.exercise) {
      await openExercise(overlay.exercise)
    }
  }

  return {
    handleOverlayClick,
  }
}
