import { describe, expect, it, vi } from 'vitest'
import { createExerciseBridge, extractExerciseRuntimePaths } from '../runtime'

describe('extractExerciseRuntimePaths', () => {
  it('extracts engine and dp paths from html', () => {
    const html = `
      <script>
        A5.ENGINE_ROOT = 'eiuasset://localhost/tmp/engine/';
        A5.SKIN_URL = "eiuasset://localhost/tmp/dp/";
      </script>
    `

    expect(extractExerciseRuntimePaths(html)).toEqual({
      engine: 'eiuasset://localhost/tmp/engine/',
      dp: 'eiuasset://localhost/tmp/dp/',
    })
  })

  it('returns empty paths when markers are missing', () => {
    expect(extractExerciseRuntimePaths('<html><body>noop</body></html>')).toEqual({})
  })
})

describe('createExerciseBridge', () => {
  it('returns fallback init payload with runtime paths', () => {
    const source = {
      postMessage: vi.fn(),
    }

    const iframe = document.createElement('iframe')
    Object.defineProperty(iframe, 'contentWindow', {
      configurable: true,
      value: window,
    })

    const detach = createExerciseBridge({
      iframe,
      getContext: () => ({
        resourceId: 'RE_000106',
        title: 'exercise',
        paths: {
          engine: 'eiuasset://localhost/tmp/engine/',
          dp: 'eiuasset://localhost/tmp/dp/',
        },
      }),
    })

    window.dispatchEvent(
      new MessageEvent('message', {
        data: JSON.stringify({
          id: 371401,
          method: 'default::sendMessageToContainer',
          params: { type: 'init' },
        }),
        source: source as unknown as MessageEventSource,
      }),
    )

    expect(source.postMessage).toHaveBeenCalledTimes(1)
    const raw = source.postMessage.mock.calls[0]?.[0]
    const response = JSON.parse(String(raw))
    expect(response.result.paths).toEqual({
      engine: 'eiuasset://localhost/tmp/engine/',
      dp: 'eiuasset://localhost/tmp/dp/',
    })

    detach()
  })

  it('accepts opaque-origin hello handshake when source identity mismatches', () => {
    const postMessageSpy = vi.spyOn(window, 'postMessage').mockImplementation(() => {})
    const onLog = vi.fn()

    const iframe = document.createElement('iframe')
    Object.defineProperty(iframe, 'contentWindow', {
      configurable: true,
      value: { postMessage: vi.fn() },
    })

    const detach = createExerciseBridge({
      iframe,
      getContext: () => ({
        resourceId: 'RE_000201',
      }),
      onLog,
    })

    window.dispatchEvent(
      new MessageEvent('message', {
        data: {
          type: 'hello',
          id: 'hello-1',
        },
        source: window,
        origin: 'null',
      }),
    )

    expect(postMessageSpy).toHaveBeenCalled()
    const [payload] = postMessageSpy.mock.calls[0] ?? []
    expect(payload).toMatchObject({
      type: 'hello-ack',
      id: 'hello-1',
    })
    expect(onLog).toHaveBeenCalledWith(
      'bridge: accepting opaque-origin handshake fallback',
      expect.objectContaining({
        origin: 'null',
        type: 'hello',
      }),
    )

    detach()
    postMessageSpy.mockRestore()
  })
})
