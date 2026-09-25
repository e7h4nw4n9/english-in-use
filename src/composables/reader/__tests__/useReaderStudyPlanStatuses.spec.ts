import { describe, expect, it, vi } from 'vitest'
import { defineComponent, h, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { getBookStudyPlanStatuses } from '@/lib/api/studyPlan'
import type { BookMetadata } from '@/types'
import { useReaderStudyPlanStatuses } from '../useReaderStudyPlanStatuses'

vi.mock('@/lib/api/studyPlan', () => ({
  getBookStudyPlanStatuses: vi.fn(),
}))

const metadata: BookMetadata = {
  toc: [
    {
      key: 'chapter',
      title: 'Chapter',
      children: [
        { key: 'RE_1', title: 'One', startPage: '1' },
        { key: 'RE_2', title: 'Two', startPage: '2' },
      ],
    },
  ],
  pages: {
    '1': { label: '1', image_path: '1.jpg', resource_id: 'RE_1' },
    '2': { label: '2', image_path: '2.jpg', resource_id: 'RE_2' },
  },
  pageLabels: ['1', '2'],
  pageWidth: 800,
  pageHeight: 1200,
}

describe('useReaderStudyPlanStatuses', () => {
  it('loads plan states once and defaults missing units to unplanned', async () => {
    vi.mocked(getBookStudyPlanStatuses).mockResolvedValue([
      { resourceId: 'RE_1', status: 'active' },
    ])
    const productCode = ref('book')
    const metadataRef = ref<BookMetadata | null>(metadata)
    const refreshVersion = ref(0)
    let statuses!: ReturnType<typeof useReaderStudyPlanStatuses>['unitStatuses']
    const Harness = defineComponent({
      setup() {
        statuses = useReaderStudyPlanStatuses({
          productCode,
          metadata: metadataRef,
          refreshVersion,
        }).unitStatuses
        return () => h('div')
      },
    })

    const wrapper = mount(Harness)
    await flushPromises()

    expect(getBookStudyPlanStatuses).toHaveBeenCalledOnce()
    expect(statuses.value).toEqual({ RE_1: 'active', RE_2: 'unplanned' })
    wrapper.unmount()
  })
})
