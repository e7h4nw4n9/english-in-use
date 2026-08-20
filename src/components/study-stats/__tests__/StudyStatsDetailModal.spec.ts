import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import StudyStatsDetailModal from '../StudyStatsDetailModal.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}))

describe('StudyStatsDetailModal', () => {
  it('provides edit and delete actions for every session', async () => {
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
      visitedUnits: [],
      startAt: '2026-08-10T10:00:00Z',
      endAt: '2026-08-10T10:01:00Z',
      duration: 60,
    }
    const wrapper = mount(StudyStatsDetailModal, {
      props: {
        open: true,
        loading: false,
        date: '2026-08-10',
        sessions: [session],
        mutatingSessionId: null,
      },
      global: {
        stubs: {
          'a-modal': { template: '<div><slot /></div>' },
          'a-spin': { template: '<div><slot /></div>' },
          'a-button': { template: '<button @click="$emit(\'click\')"><slot /></button>' },
        },
      },
    })

    const buttons = wrapper.findAll('.row-actions button')
    expect(wrapper.findAll('.row-actions .operation-button')).toHaveLength(2)
    await buttons[0].trigger('click')
    await buttons[1].trigger('click')

    expect(wrapper.emitted('edit')?.[0]).toEqual([session])
    expect(wrapper.emitted('delete')?.[0]).toEqual([session])
  })
})
