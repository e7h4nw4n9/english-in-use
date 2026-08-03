import { afterEach, vi } from 'vitest'
import { config, enableAutoUnmount } from '@vue/test-utils'

enableAutoUnmount(afterEach)

vi.mock('@tauri-apps/plugin-log', () => ({
  debug: vi.fn().mockResolvedValue(undefined),
  error: vi.fn().mockResolvedValue(undefined),
  info: vi.fn().mockResolvedValue(undefined),
  trace: vi.fn().mockResolvedValue(undefined),
  warn: vi.fn().mockResolvedValue(undefined),
}))

config.global.config.warnHandler = (message) => {
  if (
    message.startsWith('Failed to resolve component: a-') ||
    message.startsWith('Failed setting prop "size" on <input>')
  ) {
    return
  }
  console.warn(`[Vue warn]: ${message}`)
}

Object.defineProperty(window, 'matchMedia', {
  configurable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

Object.defineProperties(HTMLMediaElement.prototype, {
  play: {
    configurable: true,
    value: vi.fn().mockResolvedValue(undefined),
  },
  pause: {
    configurable: true,
    value: vi.fn(),
  },
  load: {
    configurable: true,
    value: vi.fn(),
  },
})
