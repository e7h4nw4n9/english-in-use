import { describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ReaderStudyTimerFloat from '../ReaderStudyTimerFloat.vue'

vi.mock('vue-i18n', async (importOriginal) => {
  const actual = await importOriginal<typeof import('vue-i18n')>()
  return {
    ...actual,
    useI18n: () => ({
      t: (key: string) => key,
    }),
  }
})

vi.mock('ant-design-vue', () => ({
  theme: {
    useToken: () => ({
      token: {
        value: {
          colorBgElevated: '#ffffff',
          colorBorderSecondary: '#d9d9d9',
          colorText: '#111111',
        },
      },
    }),
  },
}))

function mountTimer(props: Record<string, any> = {}) {
  return mount(ReaderStudyTimerFloat, {
    props: {
      visible: true,
      timerStatus: 'idle',
      timerDisplay: '00:00:00',
      ...props,
    },
    global: {
      stubs: {
        'a-button': {
          props: ['disabled', 'title'],
          emits: ['click'],
          template:
            '<button :disabled="disabled" :title="title" @click="$emit(\'click\', $event)"><slot name="icon" /><slot /></button>',
        },
        PauseCircleOutlined: true,
        PlayCircleOutlined: true,
        RedoOutlined: true,
        SaveOutlined: true,
        MenuFoldOutlined: true,
      },
    },
  })
}

describe('ReaderStudyTimerFloat', () => {
  it('does not render when visible is false', () => {
    const wrapper = mountTimer({ visible: false })
    expect(wrapper.find('[data-testid="timer-expanded-row"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="timer-collapsed-chip"]').exists()).toBe(false)
  })

  it('emits start/pause/resume based on current timer status', async () => {
    const wrapper = mountTimer({ timerStatus: 'idle' })

    await wrapper.get('[data-testid="timer-main-button"]').trigger('click')
    expect(wrapper.emitted('timerStart')).toHaveLength(1)

    await wrapper.setProps({ timerStatus: 'running' })
    await wrapper.get('[data-testid="timer-main-button"]').trigger('click')
    expect(wrapper.emitted('timerPause')).toHaveLength(1)

    await wrapper.setProps({ timerStatus: 'paused' })
    await wrapper.get('[data-testid="timer-main-button"]').trigger('click')
    expect(wrapper.emitted('timerResume')).toHaveLength(1)
  })

  it('emits restart and stop-save actions', async () => {
    const wrapper = mountTimer({ timerStatus: 'paused' })

    await wrapper.get('[data-testid="timer-reset-button"]').trigger('click')
    expect(wrapper.emitted('timerRestart')).toHaveLength(1)

    await wrapper.get('[data-testid="timer-save-button"]').trigger('click')
    expect(wrapper.emitted('timerStopSave')).toHaveLength(1)
  })

  it('collapses to sidebar chip and can expand back', async () => {
    const wrapper = mountTimer({ timerStatus: 'running', timerDisplay: '01:02:03' })

    expect(wrapper.text()).not.toContain('studyTimer.title')
    expect(wrapper.get('[data-testid="timer-expanded-row"]').text()).toContain('01:02:03')

    await wrapper.get('[data-testid="timer-collapse-button"]').trigger('click')
    expect(wrapper.find('[data-testid="timer-expanded-row"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="timer-collapsed-chip"]').text()).toContain('01:02:03')

    await wrapper.get('[data-testid="timer-collapsed-chip"]').trigger('click')
    expect(wrapper.find('[data-testid="timer-expanded-row"]').exists()).toBe(true)
  })
})
