import { describe, expect, it } from 'vitest'
import { ref } from 'vue'
import { useReaderPagination } from '../useReaderPagination'

describe('useReaderPagination', () => {
  it('keeps current page as left page in single mode', () => {
    const pageLabels = ref(['12', '13', '14'])
    const currentPageLabel = ref('13')
    const viewMode = ref<'single' | 'spread'>('single')
    const spreadOffset = ref(0)

    const pagination = useReaderPagination({
      pageLabels,
      currentPageLabel,
      viewMode,
      spreadOffset,
    })

    expect(pagination.leftPageLabel.value).toBe('13')
    expect(pagination.rightPageLabel.value).toBe('')
    expect(pagination.displayIndex.value).toBe(1)
  })

  it('aligns odd page to right side in spread mode', () => {
    const pageLabels = ref(['1', '2', '3', '4'])
    const currentPageLabel = ref('1')
    const viewMode = ref<'single' | 'spread'>('spread')
    const spreadOffset = ref(0)

    const pagination = useReaderPagination({
      pageLabels,
      currentPageLabel,
      viewMode,
      spreadOffset,
    })

    expect(pagination.leftPageLabel.value).toBe('')
    expect(pagination.rightPageLabel.value).toBe('1')
  })

  it('moves by two pages in spread mode', () => {
    const pageLabels = ref(['10', '11', '12', '13'])
    const currentPageLabel = ref('10')
    const viewMode = ref<'single' | 'spread'>('spread')
    const spreadOffset = ref(0)

    const pagination = useReaderPagination({
      pageLabels,
      currentPageLabel,
      viewMode,
      spreadOffset,
    })

    pagination.goForward()
    expect(currentPageLabel.value).toBe('12')

    pagination.goBack()
    expect(currentPageLabel.value).toBe('10')
  })
})
