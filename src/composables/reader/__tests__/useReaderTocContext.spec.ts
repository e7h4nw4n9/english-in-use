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
  pages: {
    '12': { label: '12', image_path: 'page12.jpg', resource_id: 'RE_U1_CONTENT' },
    '13': { label: '13', image_path: 'page13.jpg', resource_id: 'RE_U1_EXERCISE' },
    '14': { label: '14', image_path: 'page14.jpg', resource_id: 'RE_U1_TAIL' },
    '15': { label: '15', image_path: 'page15.jpg', resource_id: 'RE_U2' },
    '16': { label: '16', image_path: 'page16.jpg', resource_id: 'RE_U2' },
  },
  pageLabels: ['12', '13', '14', '15', '16'],
  pageWidth: 1000,
  pageHeight: 1400,
}

function createContext({
  customMetadata = metadata,
  currentPageLabel = '12',
  leftPageLabel = '12',
  rightPageLabel = '',
  viewMode = 'single',
  fallbackUnitTitle = 'Fallback Book',
}: {
  customMetadata?: BookMetadata
  currentPageLabel?: string
  leftPageLabel?: string
  rightPageLabel?: string
  viewMode?: 'single' | 'spread'
  fallbackUnitTitle?: string
} = {}) {
  return useReaderTocContext({
    metadata: ref(customMetadata),
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
    expect(context.currentStudyPlanUnitName.value).toBe('English In Use')
    expect(context.currentStudyPlanResourceId.value).toBeNull()
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

  it('keeps one study plan context for content/exercise pages in same unit', () => {
    const contentPage = createContext({ currentPageLabel: '12', leftPageLabel: '12' })
    const exercisePage = createContext({ currentPageLabel: '13', leftPageLabel: '13' })

    expect(contentPage.currentStudyPlanUnitName.value).toBe('Unit 1')
    expect(exercisePage.currentStudyPlanUnitName.value).toBe('Unit 1')
    expect(contentPage.currentStudyPlanResourceId.value).toBe('RE_U1_CONTENT')
    expect(exercisePage.currentStudyPlanResourceId.value).toBe('RE_U1_CONTENT')
  })

  it('falls back to current page resource when study plan anchor resource is missing', () => {
    const context = createContext({
      customMetadata: {
        ...metadata,
        pages: {
          ...metadata.pages,
          '12': { label: '12', image_path: 'page12.jpg' },
        },
      },
      currentPageLabel: '13',
      leftPageLabel: '13',
    })

    expect(context.currentStudyPlanUnitName.value).toBe('Unit 1')
    expect(context.currentStudyPlanResourceId.value).toBe('RE_U1_EXERCISE')
  })
})
