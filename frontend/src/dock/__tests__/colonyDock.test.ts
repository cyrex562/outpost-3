import { describe, expect, it, vi, beforeEach } from 'vitest'
import {
  COLONY_DOCK_COMPONENT,
  COLONY_DOCK_STORAGE_KEY,
  buildDefaultColonyLayout,
  clearPersistedColonyLayout,
  loadPersistedColonyLayout,
  openBuildingDetailsPanel,
  savePersistedColonyLayout,
  type DockPanelAdder,
  type DockPanelManager,
} from '@/dock/colonyDock'

describe('buildDefaultColonyLayout (issue #321)', () => {
  it('adds the six always-present panels exactly once, each with the registered component id', () => {
    const added: { id: string; component: string }[] = []
    const fakeApi: DockPanelAdder = {
      addPanel: (opts) => {
        added.push({ id: opts.id, component: opts.component })
      },
    }

    buildDefaultColonyLayout(fakeApi)

    // `buildingDetails` is deliberately excluded — it only appears once a
    // building is clicked (issue #322), not in the default arrangement.
    const defaultPanelIds = Object.values(COLONY_DOCK_COMPONENT).filter(
      (id) => id !== COLONY_DOCK_COMPONENT.buildingDetails,
    )

    expect(added).toHaveLength(6)
    expect(new Set(added.map((p) => p.id)).size).toBe(6)
    for (const p of added) {
      expect(p.id).toBe(p.component)
    }
    expect(added.map((p) => p.id)).toEqual(expect.arrayContaining(defaultPanelIds))
    expect(added.map((p) => p.id)).not.toContain(COLONY_DOCK_COMPONENT.buildingDetails)
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

describe('openBuildingDetailsPanel (issue #322)', () => {
  /** `seedIds` pre-registers panels (e.g. `buildings`) as already open, the
   * way a real dock's default layout would have them — `openBuildingDetailsPanel`
   * looks up the buildings panel via `getPanel` before positioning itself
   * relative to it. */
  function fakeManager(
    seedIds: string[] = [],
  ): DockPanelManager & { added: { id: string; component: string; params?: unknown; position?: unknown }[] } {
    const panels = new Map<string, { api: { updateParameters: ReturnType<typeof vi.fn>; setTitle: ReturnType<typeof vi.fn>; setActive: ReturnType<typeof vi.fn> } }>()
    const added: { id: string; component: string; params?: unknown; position?: unknown }[] = []
    for (const id of seedIds) {
      panels.set(id, { api: { updateParameters: vi.fn(), setTitle: vi.fn(), setActive: vi.fn() } })
    }
    return {
      added,
      addPanel: (opts) => {
        added.push(opts)
        const handle = { api: { updateParameters: vi.fn(), setTitle: vi.fn(), setActive: vi.fn() } }
        panels.set(opts.id, handle)
        return handle
      },
      getPanel: (id) => panels.get(id),
    }
  }

  it('adds a new panel docked alongside buildings when none is open yet', () => {
    const api = fakeManager([COLONY_DOCK_COMPONENT.buildings])

    openBuildingDetailsPanel(api, 'colony_hq')

    expect(api.added).toEqual([
      {
        id: COLONY_DOCK_COMPONENT.buildingDetails,
        title: 'colony_hq',
        component: COLONY_DOCK_COMPONENT.buildingDetails,
        params: { buildingType: 'colony_hq' },
        position: { referencePanel: COLONY_DOCK_COMPONENT.buildings, direction: 'right' },
      },
    ])
  })

  it('adds an unpositioned panel when the buildings panel is not currently open', () => {
    // Panels are user-closeable and layouts persist across sessions, so the
    // buildings list may not be there — `addPanel` with a `referencePanel`
    // that doesn't resolve to an open panel throws in real dockview-core, so
    // this must fall back rather than positioning relative to it.
    const api = fakeManager()

    openBuildingDetailsPanel(api, 'colony_hq')

    expect(api.added).toEqual([
      {
        id: COLONY_DOCK_COMPONENT.buildingDetails,
        title: 'colony_hq',
        component: COLONY_DOCK_COMPONENT.buildingDetails,
        params: { buildingType: 'colony_hq' },
      },
    ])
  })

  it('retargets the existing panel instead of adding a second one', () => {
    const api = fakeManager([COLONY_DOCK_COMPONENT.buildings])

    openBuildingDetailsPanel(api, 'colony_hq')
    openBuildingDetailsPanel(api, 'research_lab')

    expect(api.added).toHaveLength(1)
    const panel = api.getPanel(COLONY_DOCK_COMPONENT.buildingDetails)
    expect(panel?.api.updateParameters).toHaveBeenCalledWith({ buildingType: 'research_lab' })
    expect(panel?.api.setTitle).toHaveBeenCalledWith('research_lab')
    expect(panel?.api.setActive).toHaveBeenCalled()
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
