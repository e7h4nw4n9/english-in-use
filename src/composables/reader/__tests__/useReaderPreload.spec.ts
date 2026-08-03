import { defineComponent, ref } from 'vue'
import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useReaderPreload } from '../useReaderPreload'

const { resolvePageResource, resolveBookAsset } = vi.hoisted(() => ({
  resolvePageResource: vi.fn(),
  resolveBookAsset: vi.fn(),
}))

vi.mock('@/lib/api/books', () => ({
  resolvePageResource,
  resolveBookAsset,
}))

describe('useReaderPreload', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    resolvePageResource.mockReset()
    resolveBookAsset.mockReset()
  })

  afterEach(() => vi.useRealTimers())

  it('limits page preloading to two concurrent requests and deduplicates in-flight tasks', async () => {
    let activeRequests = 0
    let maxActiveRequests = 0
    const releases: Array<() => void> = []
    resolvePageResource.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          activeRequests++
          maxActiveRequests = Math.max(maxActiveRequests, activeRequests)
          releases.push(() => {
            activeRequests--
            resolve('/cached/page.jpg')
          })
        }),
    )

    let preload!: ReturnType<typeof useReaderPreload>
    const wrapper = mount(
      defineComponent({
        setup() {
          preload = useReaderPreload({
            currentBook: ref({ product_code: 'demo' } as any),
            metadata: ref({ pages: {} } as any),
            displayIndex: ref(1),
            viewMode: ref('single'),
            sortedPageLabels: ref(['1', '2', '3', '4', '5']),
          })
          return () => null
        },
      }),
    )

    preload.triggerPreload()
    preload.triggerPreload()
    vi.advanceTimersByTime(1000)
    await Promise.resolve()

    expect(resolvePageResource).toHaveBeenCalledTimes(2)
    expect(maxActiveRequests).toBe(2)

    releases.shift()?.()
    await Promise.resolve()
    await Promise.resolve()

    expect(resolvePageResource).toHaveBeenCalledTimes(3)
    expect(maxActiveRequests).toBe(2)
    releases.splice(0).forEach((release) => release())
    wrapper.unmount()
  })

  it('keeps reset generations isolated without exceeding global concurrency', async () => {
    let activeRequests = 0
    let maxActiveRequests = 0
    const releases: Array<() => void> = []
    resolvePageResource.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          activeRequests++
          maxActiveRequests = Math.max(maxActiveRequests, activeRequests)
          releases.push(() => {
            activeRequests--
            resolve('/cached/page.jpg')
          })
        }),
    )

    let preload!: ReturnType<typeof useReaderPreload>
    const wrapper = mount(
      defineComponent({
        setup() {
          preload = useReaderPreload({
            currentBook: ref({ product_code: 'demo' } as any),
            metadata: ref({ pages: {} } as any),
            displayIndex: ref(1),
            viewMode: ref('single'),
            sortedPageLabels: ref(['1', '2', '3', '4', '5']),
          })
          return () => null
        },
      }),
    )

    preload.triggerPreload()
    vi.advanceTimersByTime(1000)
    await Promise.resolve()
    expect(resolvePageResource).toHaveBeenCalledTimes(2)

    preload.resetPreload()
    preload.triggerPreload()
    vi.advanceTimersByTime(1000)
    await Promise.resolve()
    expect(resolvePageResource).toHaveBeenCalledTimes(2)

    releases.splice(0, 2).forEach((release) => release())
    await Promise.resolve()
    await Promise.resolve()
    expect(resolvePageResource).toHaveBeenCalledTimes(4)
    expect(maxActiveRequests).toBe(2)

    releases.splice(0).forEach((release) => release())
    wrapper.unmount()
  })
})
