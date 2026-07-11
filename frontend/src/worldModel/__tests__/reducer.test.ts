/**
 * Unit tests for the pure world-model reducer.
 *
 * These tests run in Vitest with a jsdom environment.
 * They cover every ServerEvent kind to ensure the reducer is total
 * and produces the correct derived state.
 */

import { describe, it, expect } from 'vitest'
import { applyEvent, hydrateFromSnapshot } from '../reducer'
import type { WorldState } from '../model'
import { EMPTY_WORLD_STATE } from '../model'
import type { WorldSnapshot } from '@/types/api'

const COLONY_ID = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee'
const PROJECT_ID = 'ffffffff-1111-2222-3333-444444444444'

function stateWithColony(): WorldState {
  return {
    ...EMPTY_WORLD_STATE,
    sol: 5,
    colonies: {
      [COLONY_ID]: {
        id: COLONY_ID,
        name: 'Alpha Base',
        population: 100,
        stability: 0.8,
        available_labour: 10,
        buildings: [],
        active_projects: [],
        commodity_pool: [],
        active_construction: [],
      },
    },
  }
}

describe('hydrateFromSnapshot', () => {
  it('sets sol and month from snapshot', () => {
    const snap: WorldSnapshot = { sol: 3, month: 1, colonies: [] }
    const state = hydrateFromSnapshot(snap)
    expect(state.sol).toBe(3)
    expect(state.month).toBe(1)
  })

  it('populates colonies from snapshot', () => {
    const snap: WorldSnapshot = {
      sol: 1,
      month: 0,
      colonies: [
        {
          id: COLONY_ID,
          name: 'Alpha',
          population: 50,
          stability: 0.8,
          available_labour: 10,
          commodity_pool: [['food', 20]],
          buildings: ['habitat'],
          active_construction: ['lab'],
        },
      ],
    }
    const state = hydrateFromSnapshot(snap)
    expect(state.colonies[COLONY_ID]).toBeDefined()
    expect(state.colonies[COLONY_ID]!.name).toBe('Alpha')
    expect(state.colonies[COLONY_ID]!.stability).toBe(0.8)
    expect(state.colonies[COLONY_ID]!.available_labour).toBe(10)
    expect(state.colonies[COLONY_ID]!.buildings).toEqual(['habitat'])
    expect(state.colonies[COLONY_ID]!.active_projects).toHaveLength(1)
    expect(state.colonies[COLONY_ID]!.active_projects[0]!.building_type).toBe('lab')
  })

  it('available_labour is never negative on hydrate', () => {
    const snap: WorldSnapshot = {
      sol: 0,
      month: 0,
      colonies: [
        {
          id: COLONY_ID,
          name: 'Alpha',
          population: 10,
          stability: 0.5,
          available_labour: -5,
          commodity_pool: [],
          buildings: [],
          active_construction: [],
        },
      ],
    }
    const state = hydrateFromSnapshot(snap)
    expect(state.colonies[COLONY_ID]!.available_labour).toBeGreaterThanOrEqual(0)
  })

  it('clears notifications on hydrate', () => {
    const snap: WorldSnapshot = { sol: 0, month: 0, colonies: [] }
    const state = hydrateFromSnapshot(snap)
    expect(state.notifications).toHaveLength(0)
  })
})

describe('applyEvent — colony_sol_advanced', () => {
  it('increments sol', () => {
    const state = applyEvent(EMPTY_WORLD_STATE, { kind: 'colony_sol_advanced', sol: 7 })
    expect(state.sol).toBe(7)
  })
})

describe('applyEvent — strategic_month_advanced', () => {
  it('increments month', () => {
    const state = applyEvent(EMPTY_WORLD_STATE, { kind: 'strategic_month_advanced', month: 2 })
    expect(state.month).toBe(2)
  })
})

describe('applyEvent — colony_founded', () => {
  it('adds a new colony to the world', () => {
    const state = applyEvent(EMPTY_WORLD_STATE, {
      kind: 'colony_founded',
      colony_id: COLONY_ID,
      name: 'Alpha Base',
      starting_population: 100,
    })
    expect(state.colonies[COLONY_ID]).toBeDefined()
    expect(state.colonies[COLONY_ID]!.name).toBe('Alpha Base')
    expect(state.colonies[COLONY_ID]!.population).toBe(100)
  })

  it('initialises stability to 1.0', () => {
    const state = applyEvent(EMPTY_WORLD_STATE, {
      kind: 'colony_founded',
      colony_id: COLONY_ID,
      name: 'Alpha Base',
      starting_population: 50,
    })
    expect(state.colonies[COLONY_ID]!.stability).toBe(1.0)
  })
})

describe('applyEvent — construction_queued', () => {
  it('adds project to colony active_projects', () => {
    const state = applyEvent(stateWithColony(), {
      kind: 'construction_queued',
      colony_id: COLONY_ID,
      building_type: 'greenhouse',
      project_id: PROJECT_ID,
    })
    expect(state.colonies[COLONY_ID]!.active_projects).toHaveLength(1)
    expect(state.colonies[COLONY_ID]!.active_projects[0]!.project_id).toBe(PROJECT_ID)
  })

  it('returns unchanged state for unknown colony', () => {
    const state = applyEvent(EMPTY_WORLD_STATE, {
      kind: 'construction_queued',
      colony_id: COLONY_ID,
      building_type: 'greenhouse',
      project_id: PROJECT_ID,
    })
    expect(state).toBe(EMPTY_WORLD_STATE)
  })
})

describe('applyEvent — construction_cancelled', () => {
  it('removes project from active_projects', () => {
    const base = applyEvent(stateWithColony(), {
      kind: 'construction_queued',
      colony_id: COLONY_ID,
      building_type: 'greenhouse',
      project_id: PROJECT_ID,
    })
    const state = applyEvent(base, {
      kind: 'construction_cancelled',
      colony_id: COLONY_ID,
      project_id: PROJECT_ID,
      refund: [['steel', 10]],
    })
    expect(state.colonies[COLONY_ID]!.active_projects).toHaveLength(0)
  })
})

describe('applyEvent — building_constructed', () => {
  it('adds building to colony buildings list', () => {
    const state = applyEvent(stateWithColony(), {
      kind: 'building_constructed',
      colony_id: COLONY_ID,
      building_type: 'greenhouse',
    })
    expect(state.colonies[COLONY_ID]!.buildings).toContain('greenhouse')
  })
})

describe('applyEvent — needs_resolved', () => {
  it('updates stability and population', () => {
    const state = applyEvent(stateWithColony(), {
      kind: 'needs_resolved',
      colony_id: COLONY_ID,
      composite_satisfaction: 0.9,
      stability_delta: 0.05,
      population_delta: 2,
    })
    expect(state.colonies[COLONY_ID]!.stability).toBeCloseTo(0.85)
    expect(state.colonies[COLONY_ID]!.population).toBe(102)
  })

  it('clamps stability to [0, 1]', () => {
    const highStab = {
      ...stateWithColony(),
      colonies: {
        [COLONY_ID]: { ...stateWithColony().colonies[COLONY_ID]!, stability: 0.98 },
      },
    }
    const state = applyEvent(highStab, {
      kind: 'needs_resolved',
      colony_id: COLONY_ID,
      composite_satisfaction: 1.0,
      stability_delta: 0.1,
      population_delta: 0,
    })
    expect(state.colonies[COLONY_ID]!.stability).toBeLessThanOrEqual(1)
  })

  it('adds notable notification on low satisfaction', () => {
    const state = applyEvent(stateWithColony(), {
      kind: 'needs_resolved',
      colony_id: COLONY_ID,
      composite_satisfaction: 0.2,
      stability_delta: -0.05,
      population_delta: -1,
    })
    expect(state.notifications).toHaveLength(1)
    expect(state.notifications[0]!.tier).toBe('notable')
  })

  it('does not add notification on adequate satisfaction', () => {
    const state = applyEvent(stateWithColony(), {
      kind: 'needs_resolved',
      colony_id: COLONY_ID,
      composite_satisfaction: 0.8,
      stability_delta: 0,
      population_delta: 0,
    })
    expect(state.notifications).toHaveLength(0)
  })
})

describe('applyEvent — research_produced', () => {
  it('accumulates research total', () => {
    const s1 = applyEvent(EMPTY_WORLD_STATE, {
      kind: 'research_produced',
      colony_id: COLONY_ID,
      amount: 5.0,
    })
    const s2 = applyEvent(s1, {
      kind: 'research_produced',
      colony_id: COLONY_ID,
      amount: 3.5,
    })
    expect(s2.research_total).toBeCloseTo(8.5)
  })
})

describe('applyEvent — production_shortfall', () => {
  it('adds a notable notification', () => {
    const state = applyEvent(stateWithColony(), {
      kind: 'production_shortfall',
      colony_id: COLONY_ID,
      building_type: 'mine',
      scale: 0.5,
      reason: 'Labour',
    })
    expect(state.notifications).toHaveLength(1)
    expect(state.notifications[0]!.message).toContain('mine')
  })
})

describe('applyEvent — pass-through events', () => {
  const passThrough = [
    { kind: 'directive_set' as const, colony_id: COLONY_ID, directive_id: 'dir-1' },
    { kind: 'directive_removed' as const, directive_id: 'dir-1' },
    { kind: 'directive_fired' as const, colony_id: COLONY_ID, directive_id: 'dir-1' },
    { kind: 'manual_override_changed' as const, colony_id: COLONY_ID, enabled: true },
  ]
  for (const ev of passThrough) {
    it(`returns unchanged state for ${ev.kind}`, () => {
      const base = stateWithColony()
      const state = applyEvent(base, ev)
      expect(state).toBe(base)
    })
  }
})

describe('reducer immutability', () => {
  it('does not mutate the original state object', () => {
    const base = stateWithColony()
    const orig = JSON.stringify(base)
    applyEvent(base, { kind: 'colony_sol_advanced', sol: 99 })
    expect(JSON.stringify(base)).toBe(orig)
  })
})
