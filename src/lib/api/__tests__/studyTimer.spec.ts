import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { deleteStudySession, updateStudySession } from '../studyTimer'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('study timer mutation API', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(invoke).mockResolvedValue({ success: true, sessionId: 7 })
  })

  it('passes update payload and delete identifier to Tauri commands', async () => {
    const payload = { sessionId: 7, duration: 3661, assignedResourceId: 'RE_2' }

    await updateStudySession(payload)
    await deleteStudySession(7)

    expect(invoke).toHaveBeenNthCalledWith(1, 'update_study_session', { payload })
    expect(invoke).toHaveBeenNthCalledWith(2, 'delete_study_session', { sessionId: 7 })
  })
})
