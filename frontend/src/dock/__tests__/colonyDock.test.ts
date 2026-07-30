import { describe, expect, it, beforeEach } from 'vitest'
import {
  COLONY_DOCK_COMPONENT,
  COLONY_DOCK_STORAGE_KEY,
  buildDefaultColonyLayout,
  clearPersistedColonyLayout,
  loadPersistedColonyLayout,
  savePersistedColonyLayout,
  type DockPanelAdder,
} from '@/dock/colonyDock'

describe('buildDefaultColonyLayout (issue #321)', () => {
  it('adds all six panels exactly once, each with the registered component id', () => {
    const added: { id: string; component: string }[] = []
    const fakeApi: DockPanelAdder = {
      addPanel: (opts) => {
        added.push({ id: opts.id, component: opts.component })
      },
    }

    buildDefaultColonyLayout(fakeApi)

    expect(added).toHaveLength(6)
    expect(new Set(added.map((p) => p.id)).size).toBe(6)
    for (const p of added) {
      expect(p.id).toBe(p.component)
    }
    expect(added.map((p) => p.id)).toEqual(
      expect.arrayContaining(Object.values(COLONY_DOCK_COMPONENT)),
    )
  })

  it('mirrors the pre-#321 3-column shape: vitals/utilities/commodities stacked left, buildings/queue stacked center, alerts on the right', () => {
    const positions: Record<string, { referencePanel: string; direction: string } | undefined> = {}
    const fakeApi: DockPanelAdder = {
      addPanel: (opts) => {
        positions[opts.id] = opts.position
      },
    }

    buildDefaultColonyLayout(fakeApi)
    const c = COLONY_DOCK_COMPONENT

    // Left column: vitals is the root panel; utilities/commodities stack below it.
    expect(positions[c.vitalStats]).toBeUndefined()
    expect(positions[c.utilities]).toEqual({ referencePanel: c.vitalStats, direction: 'below' })
    expect(positions[c.commodities]).toEqual({ referencePanel: c.utilities, direction: 'below' })

    // Center column: buildings sits to the right of vitals; the queue stacks below it.
    expect(positions[c.buildings]).toEqual({ referencePanel: c.vitalStats, direction: 'right' })
    expect(positions[c.constructionQueue]).toEqual({ referencePanel: c.buildings, direction: 'below' })

    // Right column: alerts sits to the right of the center column.
    expect(positions[c.alerts]).toEqual({ referencePanel: c.buildings, direction: 'right' })
  })
})

describe('colony dock layout persistence (issue #321)', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('returns null when nothing is persisted', () => {
    expect(loadPersistedColonyLayout()).toBeNull()
  })

  it('round-trips a serialized layout through save/load', () => {
    const layout = { grid: { root: { type: 'leaf', data: {} } } }
    savePersistedColonyLayout(layout)
    expect(loadPersistedColonyLayout()).toEqual(layout)
  })

  it('falls back to null on a corrupt persisted entry rather than throwing', () => {
    window.localStorage.setItem(COLONY_DOCK_STORAGE_KEY, '{not json')
    expect(loadPersistedColonyLayout()).toBeNull()
  })

  it('clears the persisted layout', () => {
    savePersistedColonyLayout({ some: 'layout' })
    clearPersistedColonyLayout()
    expect(loadPersistedColonyLayout()).toBeNull()
  })
})
