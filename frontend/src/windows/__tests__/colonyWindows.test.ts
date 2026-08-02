import { describe, expect, it, beforeEach } from 'vitest'
import {
  COLONY_WINDOW,
  COLONY_WINDOW_IDS,
  COLONY_WINDOW_TITLES,
  COLONY_WINDOW_DEFAULT_RECT,
  colonyWindowStorageKey,
  clearAllColonyWindowGeometry,
  loadPersistedOpenWindowIds,
  savePersistedOpenWindowIds,
  clearPersistedOpenWindowIds,
} from '@/windows/colonyWindows'

describe('colony window registry', () => {
  it('defines exactly six windows, each with a title and a default rect', () => {
    expect(COLONY_WINDOW_IDS).toHaveLength(6)
    expect(new Set(COLONY_WINDOW_IDS).size).toBe(6)
    for (const id of COLONY_WINDOW_IDS) {
      expect(COLONY_WINDOW_TITLES[id]).toBeTruthy()
      expect(COLONY_WINDOW_DEFAULT_RECT[id]).toMatchObject({
        x: expect.any(Number),
        y: expect.any(Number),
        w: expect.any(Number),
        h: expect.any(Number),
      })
    }
  })

  it('gives every window a distinct storage key', () => {
    const keys = COLONY_WINDOW_IDS.map(colonyWindowStorageKey)
    expect(new Set(keys).size).toBe(keys.length)
  })
})

describe('open-window persistence', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('returns null when nothing is persisted', () => {
    expect(loadPersistedOpenWindowIds()).toBeNull()
  })

  it('round-trips a set of open window ids', () => {
    const ids = [COLONY_WINDOW.vitalStats, COLONY_WINDOW.alerts]
    savePersistedOpenWindowIds(ids)
    expect(loadPersistedOpenWindowIds()).toEqual(ids)
  })

  it('falls back to null on a corrupt persisted entry rather than throwing', () => {
    window.localStorage.setItem('outpost3.colony-view.open-windows.v1', '{not json')
    expect(loadPersistedOpenWindowIds()).toBeNull()
  })

  it('drops unknown ids from a persisted entry (e.g. a retired panel) instead of surfacing a ghost window', () => {
    window.localStorage.setItem(
      'outpost3.colony-view.open-windows.v1',
      JSON.stringify([COLONY_WINDOW.buildings, 'some-retired-panel']),
    )
    expect(loadPersistedOpenWindowIds()).toEqual([COLONY_WINDOW.buildings])
  })

  it('clears the persisted open-window set', () => {
    savePersistedOpenWindowIds([COLONY_WINDOW.commodities])
    clearPersistedOpenWindowIds()
    expect(loadPersistedOpenWindowIds()).toBeNull()
  })
})

describe('window geometry reset', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('clearAllColonyWindowGeometry removes every window\'s persisted rect, and nothing else', () => {
    for (const id of COLONY_WINDOW_IDS) {
      window.localStorage.setItem(colonyWindowStorageKey(id), JSON.stringify({ x: 1, y: 1, w: 1, h: 1 }))
    }
    window.localStorage.setItem('unrelated-key', 'still here')

    clearAllColonyWindowGeometry()

    for (const id of COLONY_WINDOW_IDS) {
      expect(window.localStorage.getItem(colonyWindowStorageKey(id))).toBeNull()
    }
    expect(window.localStorage.getItem('unrelated-key')).toBe('still here')
  })
})
