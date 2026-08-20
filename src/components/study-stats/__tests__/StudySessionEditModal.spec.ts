import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import StudySessionEditModal from '../StudySessionEditModal.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}))

const session = {
  id: 7,
  bookId: 1,
  bookGroup: 1,
  productCode: 'book',
  bookTitle: 'Book',
  resourceId: 'RE_1',
  unitName: 'Unit 1',
  entryResourceId: 'RE_1',
  entryUnitName: 'Unit 1',
  visitedUnits: [
    { resourceId: 'RE_1', unitName: 'Unit 1' },
    { resourceId: 'RE_2', unitName: 'Unit 2' },
  ],
  startAt: '2026-08-10T10:00:00Z',
  endAt: '2026-08-10T10:01:01Z',
  duration: 61,
}

describe('StudySessionEditModal', () => {
  it('uses the existing duration and unit when submitting', async () => {
    const wrapper = mount(StudySessionEditModal, {
      props: { open: true, session, loading: false },
      global: {
        stubs: {
          'a-modal': {
            template: '<button class="submit" @click="$emit(\'ok\')"><slot /></button>',
          },
          'a-select': { template: '<div><slot /></div>' },
          'a-select-option': { template: '<span><slot /></span>' },
          'a-input-number': { template: '<input />' },
        },
      },
    })

    await wrapper.find('.submit').trigger('click')

    expect(wrapper.emitted('confirm')?.[0]).toEqual([
      { sessionId: 7, duration: 61, assignedResourceId: 'RE_1' },
    ])
    expect(wrapper.text()).toContain('studyStats.newEndTime')
  })
})
