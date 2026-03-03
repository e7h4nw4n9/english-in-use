import { describe, expect, it } from 'vitest'
import { ref } from 'vue'
import { useReaderTocContext } from '../useReaderTocContext'
import type { BookMetadata } from '@/types'

const metadata: BookMetadata = {
  toc: [
    {
      title: 'Unit 1',
      key: 'unit1',
      startPage: '12',
      endPage: '14',
      audioFiles: [{ path: 'unit1.mp3', title: 'Unit 1 Audio' }],
      children: [
        {
          title: 'Section 1.1',
          key: 'unit1-sec1',
          startPage: '12',
          endPage: '12',
          audioFiles: [{ path: 'sec1.mp3', title: 'Section 1.1 Audio' }],
        },
        {
          title: 'Section 1.2',
          key: 'unit1-sec2',
          startPage: '13',
          endPage: '13',
        },
      ],
    },
    {
      title: 'Unit 2',
      key: 'unit2',
      startPage: '15',
      endPage: '16',
      audioFiles: [{ path: 'unit2.mp3', title: 'Unit 2 Audio' }],
    },
  ],
  exerciseToc: [],
  pages: {},
  pageLabels: ['12', '13', '14', '15', '16'],
  pageWidth: 1000,
  pageHeight: 1400,
}

function createContext({
  currentPageLabel = '12',
  leftPageLabel = '12',
  rightPageLabel = '',
  viewMode = 'single',
  fallbackUnitTitle = 'Fallback Book',
}: {
  currentPageLabel?: string
  leftPageLabel?: string
  rightPageLabel?: string
  viewMode?: 'single' | 'spread'
  fallbackUnitTitle?: string
} = {}) {
  return useReaderTocContext({
    metadata: ref(metadata),
    currentPageLabel: ref(currentPageLabel),
    leftPageLabel: ref(leftPageLabel),
    rightPageLabel: ref(rightPageLabel),
    viewMode: ref(viewMode),
    sortedPageLabels: ref(['12', '13', '14', '15', '16']),
    fallbackUnitTitle: ref(fallbackUnitTitle),
  })
}

describe('useReaderTocContext', () => {
  it('resolves deepest matching unit title', () => {
    const context = createContext({ currentPageLabel: '12', leftPageLabel: '12' })
    expect(context.currentUnitName.value).toBe('Section 1.1')
  })

  it('falls back to book title when page does not match toc range', () => {
    const context = createContext({
      currentPageLabel: '99',
      leftPageLabel: '99',
      fallbackUnitTitle: 'English In Use',
    })
    expect(context.currentUnitName.value).toBe('English In Use')
  })

  it('uses child audio first and falls back to parent audio', () => {
    const page12 = createContext({ currentPageLabel: '12', leftPageLabel: '12' })
    expect(page12.currentPageAudioFiles.value.map((audio) => audio.path)).toEqual(['sec1.mp3'])

    const page13 = createContext({ currentPageLabel: '13', leftPageLabel: '13' })
    expect(page13.currentPageAudioFiles.value.map((audio) => audio.path)).toEqual(['unit1.mp3'])
  })

  it('prefers right page audio in spread mode', () => {
    const context = createContext({
      currentPageLabel: '12',
      leftPageLabel: '12',
      rightPageLabel: '15',
      viewMode: 'spread',
    })
    expect(context.currentPageAudioFiles.value.map((audio) => audio.path)).toEqual(['unit2.mp3'])
  })
})
