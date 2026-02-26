import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useReaderStore } from '../reader'

describe('Reader Store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('should initialize with default zoom level', () => {
    const store = useReaderStore()
    expect(store.zoomLevel).toBe(1.0)
  })

  it('should zoom in', () => {
    const store = useReaderStore()
    store.zoomIn()
    expect(store.zoomLevel).toBe(1.05)
  })

  it('should zoom out', () => {
    const store = useReaderStore()
    store.zoomOut()
    expect(store.zoomLevel).toBe(0.95)
  })

  it('should reset zoom', () => {
    const store = useReaderStore()
    store.setZoomLevel(2.0)
    store.resetZoom()
    expect(store.zoomLevel).toBe(1.0)
  })

  it('should respect zoom limits', () => {
    const store = useReaderStore()
    store.setZoomLevel(10.0)
    expect(store.zoomLevel).toBe(5.0)
    store.setZoomLevel(0.1)
    expect(store.zoomLevel).toBe(0.25)
  })
})
