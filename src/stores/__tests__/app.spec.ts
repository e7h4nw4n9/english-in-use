import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAppStore } from '../app'
import * as api from '../../lib/api'

// Mock dependencies
vi.mock('@tauri-apps/plugin-log', () => ({
  info: vi.fn(),
  error: vi.fn(),
  debug: vi.fn(),
}))

vi.mock('../../lib/api', () => ({
  loadConfig: vi.fn(),
  checkConnectionStatus: vi.fn(),
  initializeDatabase: vi.fn(),
}))

vi.mock('../../i18n', () => ({
  default: {
    global: {
      t: (key: string) => key,
    },
  },
}))

describe('App Store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('initializes with default values', () => {
    const store = useAppStore()
    expect(store.config).toBeNull()
    expect(store.isLoading).toBe(true)
    expect(store.connectionStatus.r2.status).toBe('NotConfigured')
  })

  it('validates config correctly', async () => {
    const store = useAppStore()

    // Invalid config (null)
    expect(store.isConfigValid).toBe(false)

    // Invalid config (missing details)
    store.config = {
      book_source: { type: 'Local', details: { path: '' } },
      database: { type: 'SQLite', details: { path: '' } },
      system: { language: 'zh', theme: 'system', enable_auto_check: true },
    } as any
    expect(store.isConfigValid).toBe(false)

    // Valid config
    store.config = {
      book_source: { type: 'Local', details: { path: '/path/to/books' } },
      database: { type: 'SQLite', details: { path: '/path/to/db' } },
      system: { language: 'zh', theme: 'system', enable_auto_check: true },
    } as any
    expect(store.isConfigValid).toBe(true)
  })

  it('initApp loads config and initializes database', async () => {
    const mockConfig = {
      book_source: { type: 'Local', details: { path: '/path' } },
      database: { type: 'SQLite', details: { path: '/db' } },
      system: { language: 'zh', theme: 'system', enable_auto_check: false },
    }

    vi.mocked(api.loadConfig).mockResolvedValue(mockConfig as any)
    vi.mocked(api.initializeDatabase).mockResolvedValue(true)

    const store = useAppStore()
    await store.initApp()

    expect(api.loadConfig).toHaveBeenCalled()
    expect(api.initializeDatabase).toHaveBeenCalled()
    expect(store.config).toEqual(mockConfig)
    expect(store.isLoading).toBe(false)
  })
})
