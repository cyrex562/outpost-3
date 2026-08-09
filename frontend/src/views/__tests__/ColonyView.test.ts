import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick, reactive } from 'vue'
import ColonyView from '@/views/ColonyView.vue'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import { COLONY_WINDOW_TITLES } from '@/windows/colonyWindows'

const routerPush = vi.fn()
const routerReplace = vi.fn()
const routeParams = reactive<{ colonyId: string | undefined; buildingType: string | undefined }>({
  colonyId: undefined,
  buildingType: undefined,
})

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush, replace: routerReplace }),
  useRoute: () => ({ params: routeParams }),
}))

vi.mock('@/services/tauriBridge', () => ({
  isTauri: false,
  listBuildings: vi.fn().mockResolvedValue([]),
  getTechTree: vi.fn().mockResolvedValue([]),
}))

/**
 * The six panel windows are stubbed by default — these tests care about the
 * window-management shell (open/close/reopen/reset, the building-details
 * window), not each panel's own internal rendering (that's each panel
 * component's own test file). `BuildingsWindowPanel` is un-stubbed in tests
 * that need its real "view details" button to emit the event ColonyView's
 * window context listens for.
 */
const STUBS = {
  VitalStatsWindowPanel: true,
  UtilitiesWindowPanel: true,
  CommoditiesWindowPanel: true,
  BuildingsWindowPanel: true,
  ConstructionQueueWindowPanel: true,
  AlertsWindowPanel: true,
  BuildDialog: true,
}

function seedColonies(): void {
  const worldStore = useWorldStore()
  worldStore.world.colonies = {
    'colony-1': {
      id: 'colony-1',
      name: 'Alpha Base',
      population: 100,
      stability: 0.9,
      available_labour: 5,
      buildings: [],
      active_projects: [],
      commodity_pool: [],
      active_construction: [],
    },
    'colony-2': {
      id: 'colony-2',
      name: 'Beta Outpost',
      population: 50,
      stability: 0.8,
      available_labour: 2,
      buildings: [],
      active_projects: [],
      commodity_pool: [],
      active_construction: [],
    },
  }
}

function colonyScreenWithHqBuilding(overrides: Record<string, unknown> = {}) {
  return {
    colony_id: 'colony-1',
    name: 'Alpha Base',
    population: 100,
    stability: 0.9,
    morale: 0.85,
    slots_used: 1,
    slot_capacity: 5,
    labour_available: 5,
    labour_total: 10,
    labour_demanded: 4,
    labour_employed: 4,
    labour_unemployed: 1,
    resources: [],
    buildings: [
      {
        building_id: 'hq-instance-1',
        name: 'Colony HQ 1',
        building_type: 'colony_hq',
        labour_assigned: 0,
        labour_demand: 0,
        priority: 5,
        labour_lock: null,
        paused: false,
        slot_cost: 1,
        full_capacity: true,
        scale: 1.0,
        shortfall_reason: null,
        shortfall_kind: null,
        always_on: true,
        running_recipe_ids: [],
        inputs: [],
        outputs: [],
      },
    ],
    stockpile: [],
    construction_queue: [],
    manual_override: false,
    ...overrides,
  }
}

describe('ColonyView colony selection (navigation rework #7 phase 1: route param is source of truth)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    routerReplace.mockReset()
    routeParams.colonyId = undefined
    routeParams.buildingType = undefined
    window.localStorage.clear()
  })

  it('selects the colony named by the route param', () => {
    seedColonies()
    routeParams.colonyId = 'colony-2'

    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })

    expect(wrapper.get('[data-testid^="colony-detail-"]').attributes('data-testid')).toBe(
      'colony-detail-colony-2',
    )
    expect(wrapper.get('[data-testid="colony-title"]').text()).toBe('Beta Outpost')
  })

  it('falls back to gameStore.selectedColonyId when no route param is present', () => {
    seedColonies()
    const gameStore = useGameStore()
    gameStore.selectedColonyId = 'colony-2'

    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })

    expect(wrapper.get('[data-testid^="colony-detail-"]').attributes('data-testid')).toBe(
      'colony-detail-colony-2',
    )
  })

  it('auto-selects and replaces the route with the first colony when nothing is selected', () => {
    seedColonies()

    mount(ColonyView, { global: { stubs: STUBS } })

    expect(routerReplace).toHaveBeenCalledWith({ name: 'colony', params: { colonyId: 'colony-1' } })
  })

  it('returns to the planet map when the back button is clicked (map/nav plan phase A2)', async () => {
    seedColonies()
    routeParams.colonyId = 'colony-1'

    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })
    await wrapper.get('[data-testid="btn-planet-map"]').trigger('click')

    expect(routerPush).toHaveBeenCalledWith({ name: 'planet' })
  })

  it('shows the empty state when no colonies exist', () => {
    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })
    expect(wrapper.find('[data-testid="no-colonies"]').exists()).toBe(true)
  })
})

describe('ColonyView panel windows (colony details multi-window redesign)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    routerReplace.mockReset()
    routeParams.colonyId = 'colony-1'
    routeParams.buildingType = undefined
    window.localStorage.clear()
  })

  it('opens all six panel windows by default, and shows the tool palette + reset-layout button', () => {
    seedColonies()
    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })

    for (const title of Object.values(COLONY_WINDOW_TITLES)) {
      expect(wrapper.findAll('[data-testid="floating-window"]').some((w) => w.text().includes(title))).toBe(
        true,
      )
    }
    expect(wrapper.find('[data-testid="window-palette"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="btn-reset-layout"]').exists()).toBe(true)
  })

  it('closing a panel window via its own close button removes it, and the palette chip reopens it', async () => {
    seedColonies()
    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })

    // Six panel windows are open; grab the Vitals one specifically.
    const vitalsWindow = wrapper.get('[data-window-id="vital-stats"]')
    await vitalsWindow.get('[data-testid="fw-close"]').trigger('click')
    expect(wrapper.find('[data-window-id="vital-stats"]').exists()).toBe(false)

    // Its palette chip should no longer read as "open".
    const chip = wrapper.get('[data-testid="palette-toggle-vital-stats"]')
    expect(chip.classes()).not.toContain('open')

    await chip.trigger('click')
    expect(wrapper.find('[data-window-id="vital-stats"]').exists()).toBe(true)
    expect(wrapper.get('[data-testid="palette-toggle-vital-stats"]').classes()).toContain('open')
  })

  it('a palette chip also closes an already-open window (not just a one-way opener)', async () => {
    seedColonies()
    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })

    expect(wrapper.find('[data-window-id="alerts"]').exists()).toBe(true)
    await wrapper.get('[data-testid="palette-toggle-alerts"]').trigger('click')
    expect(wrapper.find('[data-window-id="alerts"]').exists()).toBe(false)
  })

  it('persists the open-window set across a remount', async () => {
    seedColonies()
    const first = mount(ColonyView, { global: { stubs: STUBS } })
    await first.get('[data-window-id="utilities"]').get('[data-testid="fw-close"]').trigger('click')
    first.unmount()

    const second = mount(ColonyView, { global: { stubs: STUBS } })
    expect(second.find('[data-window-id="utilities"]').exists()).toBe(false)
    expect(second.find('[data-window-id="commodities"]').exists()).toBe(true)
  })

  it('Reset Layout reopens every window', async () => {
    seedColonies()
    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })
    await wrapper.get('[data-window-id="buildings"]').get('[data-testid="fw-close"]').trigger('click')
    expect(wrapper.find('[data-window-id="buildings"]').exists()).toBe(false)

    await wrapper.get('[data-testid="btn-reset-layout"]').trigger('click')

    for (const title of Object.values(COLONY_WINDOW_TITLES)) {
      expect(wrapper.findAll('[data-testid="floating-window"]').some((w) => w.text().includes(title))).toBe(
        true,
      )
    }
  })
})

describe('ColonyView building-details window (issue #322)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    routerReplace.mockReset()
    routeParams.colonyId = 'colony-1'
    routeParams.buildingType = undefined
    window.localStorage.clear()
  })

  it('opens a floating window (not a route navigation) when a building requests details', async () => {
    seedColonies()
    const gameStore = useGameStore()
    gameStore.colonyScreen = colonyScreenWithHqBuilding()

    // Mount with the real BuildingsWindowPanel (and BuildingsPanel inside it)
    // so its actual "view details" button emits the real event ColonyView's
    // window context listens for. BuildingDetailsHud is stubbed since it
    // fetches over tauriBridge, which isn't mocked in this suite — only the
    // floating window shell matters here.
    const wrapper = mount(ColonyView, {
      global: {
        stubs: { ...STUBS, BuildingsWindowPanel: false, BuildingDetailsHud: true },
      },
    })
    expect(wrapper.find('[data-window-id="building-details"]').exists()).toBe(false)

    await wrapper.get('[data-testid="view-details-colony_hq"]').trigger('click')

    expect(routerPush).not.toHaveBeenCalledWith(expect.objectContaining({ name: 'facility' }))
    const win = wrapper.find('[data-window-id="building-details"]')
    expect(win.exists()).toBe(true)
    expect(win.text()).toContain('colony_hq')
  })

  it('retargets the floating window instead of opening a second one', async () => {
    seedColonies()
    const gameStore = useGameStore()
    gameStore.colonyScreen = colonyScreenWithHqBuilding({
      slots_used: 2,
      buildings: [
        ...colonyScreenWithHqBuilding().buildings,
        {
          building_id: 'lab-instance-1',
          name: 'Research Lab 1',
          building_type: 'research_lab',
          labour_assigned: 0,
          labour_demand: 0,
          priority: 5,
          labour_lock: null,
          paused: false,
          slot_cost: 1,
          full_capacity: true,
          scale: 1.0,
          shortfall_reason: null,
          shortfall_kind: null,
          always_on: false,
          running_recipe_ids: [],
          inputs: [],
          outputs: [],
        },
      ],
    })

    const wrapper = mount(ColonyView, {
      global: { stubs: { ...STUBS, BuildingsWindowPanel: false, BuildingDetailsHud: true } },
    })

    await wrapper.get('[data-testid="view-details-colony_hq"]').trigger('click')
    expect(wrapper.findAll('[data-window-id="building-details"]')).toHaveLength(1)
    expect(wrapper.get('[data-window-id="building-details"]').text()).toContain('colony_hq')

    await wrapper.get('[data-testid="view-details-research_lab"]').trigger('click')

    expect(wrapper.findAll('[data-window-id="building-details"]')).toHaveLength(1)
    expect(wrapper.get('[data-window-id="building-details"]').text()).toContain('research_lab')
  })

  it('opens the floating window for a deep-linked /colony/:colonyId/facility/:buildingType route', async () => {
    seedColonies()
    routeParams.buildingType = 'colony_hq'

    const wrapper = mount(ColonyView, {
      global: { stubs: { ...STUBS, BuildingDetailsHud: true } },
    })
    await nextTick()

    const win = wrapper.find('[data-window-id="building-details"]')
    expect(win.exists()).toBe(true)
    expect(win.text()).toContain('colony_hq')
  })

  it('closes the floating building-details window via its own close button, leaving the panel windows untouched', async () => {
    seedColonies()
    const gameStore = useGameStore()
    gameStore.colonyScreen = colonyScreenWithHqBuilding()

    const wrapper = mount(ColonyView, {
      global: {
        stubs: { ...STUBS, BuildingsWindowPanel: false, BuildingDetailsHud: true },
      },
    })
    await wrapper.get('[data-testid="view-details-colony_hq"]').trigger('click')
    expect(wrapper.find('[data-window-id="building-details"]').exists()).toBe(true)

    await wrapper.get('[data-window-id="building-details"]').get('[data-testid="fw-close"]').trigger('click')
    expect(wrapper.find('[data-window-id="building-details"]').exists()).toBe(false)
    // The six panel windows (Vitals included) are unaffected by closing a
    // different window.
    expect(wrapper.find('[data-window-id="vital-stats"]').exists()).toBe(true)
  })
})

describe('ColonyView build limits (max_instances)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    routeParams.colonyId = undefined
    routeParams.buildingType = undefined
  })

  /** A catalog entry, defaulting to a building capped at one per colony. */
  function option(overrides: Record<string, unknown> = {}) {
    return {
      id: 'colony_hq',
      name: 'Colony HQ',
      description: '',
      category: 'Services',
      slot_cost: 1,
      labor_per_turn: 2,
      construction_turns: 3,
      construction_cost: [],
      tech_prerequisite: null,
      starter_kit: true,
      max_instances: 1,
      ...overrides,
    }
  }

  function buildingRow(buildingType: string, n: number) {
    return {
      building_id: `${buildingType}-${n}`,
      name: `${buildingType} ${n}`,
      building_type: buildingType,
      labour_assigned: 0,
      labour_demand: 0,
      priority: 5,
      labour_lock: null,
      paused: false,
      slot_cost: 1,
      full_capacity: true,
    }
  }

  function queueRow(buildingType: string, n: number) {
    return {
      project_id: `p-${n}`,
      building_type: buildingType,
      turns_completed: 0,
      turns_total: 3,
      slot_cost: 1,
    }
  }

  /**
   * Mount, seed the colony screen with the given standing buildings and queued
   * projects, and open the build dialog for real (rather than stubbing it) so
   * the assertion is on what the player would actually see.
   *
   * The screen, not `ColonyState`, is what carries `building_type` — see
   * `existingCount` in the view.
   */
  async function openDialog(
    catalog: Record<string, unknown>[],
    buildings: ReturnType<typeof buildingRow>[] = [],
    queue: ReturnType<typeof queueRow>[] = [],
  ) {
    const { listBuildings } = await import('@/services/tauriBridge')
    ;(listBuildings as ReturnType<typeof vi.fn>).mockResolvedValue(catalog)

    seedColonies()
    const gameStore = useGameStore()
    gameStore.selectedColonyId = 'colony-1'
    gameStore.colonyScreen = colonyScreenWithHqBuilding({
      slots_used: buildings.length,
      slot_capacity: 20,
      buildings,
      construction_queue: queue,
    })

    const wrapper = mount(ColonyView, {
      global: { stubs: { ...STUBS, ConstructionQueueWindowPanel: false, BuildDialog: false } },
    })
    await flushPromises()
    await wrapper.get('[data-testid="btn-open-build"]').trigger('click')
    await flushPromises()
    // Guard against a vacuous pass: the assertions below check for the presence
    // or absence of one card's reason, which would also hold if the dialog
    // never opened or the catalog never loaded.
    expect(wrapper.find('[data-testid="build-dialog"]').exists()).toBe(true)
    expect(wrapper.findAll('[data-testid^="build-card-"]').length).toBeGreaterThan(0)
    return wrapper
  }

  it('disables a capped building the colony already has', async () => {
    const wrapper = await openDialog([option()], [buildingRow('colony_hq', 1)])
    // The cap now reads as a requirement badge rather than a reason line
    // (issue #423) — same information, alongside every other blocker.
    const limit = wrapper.find('[data-testid="build-req-colony_hq-limit"]')
    expect(limit.exists()).toBe(true)
    expect(limit.attributes('data-met')).toBe('false')
    expect(limit.text()).toMatch(/limit 1 per colony/i)
    expect(
      wrapper.get('[data-testid="btn-queue-colony_hq"]').attributes('disabled'),
    ).toBeDefined()
  })

  it('counts a queued copy toward the cap, not just standing buildings', async () => {
    // Queueing a second copy would look fine here but be rejected by the
    // engine, so the UI has to count the queue the same way the engine does.
    const wrapper = await openDialog([option()], [], [queueRow('colony_hq', 1)])
    const limit = wrapper.get('[data-testid="build-req-colony_hq-limit"]')
    expect(limit.attributes('data-met')).toBe('false')
    expect(limit.text()).toMatch(/limit 1 per colony/i)
  })

  it('leaves a capped building available when the colony has none yet', async () => {
    const wrapper = await openDialog([option()], [], [])
    // The badge is still listed — met requirements always show — but satisfied.
    expect(wrapper.get('[data-testid="build-req-colony_hq-limit"]').attributes('data-met')).toBe(
      'true',
    )
    expect(
      wrapper.get('[data-testid="btn-queue-colony_hq"]').attributes('disabled'),
    ).toBeUndefined()
  })

  it('leaves uncapped buildings alone however many are already built', async () => {
    const wrapper = await openDialog(
      [option({ id: 'solar_array_mk1', name: 'Solar Array Mk1', max_instances: null })],
      [buildingRow('solar_array_mk1', 1), buildingRow('solar_array_mk1', 2), buildingRow('solar_array_mk1', 3)],
    )
    expect(wrapper.find('[data-testid="build-req-solar_array_mk1-limit"]').exists()).toBe(false)
  })
})


describe('ColonyView build-catalogue filters', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    routeParams.colonyId = undefined
    routeParams.buildingType = undefined
    window.localStorage.clear()
  })

  function option(overrides: Record<string, unknown> = {}) {
    return {
      id: 'shed',
      name: 'Shed',
      description: '',
      category: 'Storage',
      slot_cost: 1,
      labor_per_turn: 1,
      construction_turns: 1,
      construction_cost: [] as [string, number][],
      tech_prerequisite: null,
      starter_kit: false,
      max_instances: null,
      ...overrides,
    }
  }

  function stock(commodityId: string, amount: number, reserved = 0) {
    return { commodity_id: commodityId, amount, capacity: null, net_per_turn: 0, reserved }
  }

  async function openDialog(
    catalog: Record<string, unknown>[],
    stockpile: ReturnType<typeof stock>[] = [],
  ) {
    const { listBuildings } = await import('@/services/tauriBridge')
    ;(listBuildings as ReturnType<typeof vi.fn>).mockResolvedValue(catalog)

    seedColonies()
    const gameStore = useGameStore()
    gameStore.selectedColonyId = 'colony-1'
    gameStore.colonyScreen = colonyScreenWithHqBuilding({
      slot_capacity: 20,
      slots_used: 0,
      stockpile,
    })

    const wrapper = mount(ColonyView, {
      global: { stubs: { ...STUBS, ConstructionQueueWindowPanel: false, BuildDialog: false } },
    })
    await flushPromises()
    await wrapper.get('[data-testid="btn-open-build"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="build-dialog"]').exists()).toBe(true)
    return wrapper
  }

  it('hides a building the colony cannot fund when the affordability filter is on', async () => {
    const wrapper = await openDialog(
      [
        option({ id: 'cheap', construction_cost: [['structural_metal', 5]] }),
        option({ id: 'dear', construction_cost: [['structural_metal', 500]] }),
      ],
      [stock('structural_metal', 100)],
    )

    expect(wrapper.find('[data-testid="build-card-dear"]').exists()).toBe(true)
    await wrapper.get('[data-testid="filter-hide-unaffordable"]').setValue(true)

    expect(wrapper.find('[data-testid="build-card-dear"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="build-card-cheap"]').exists()).toBe(true)
  })

  it('counts reserved stock as unavailable, matching what construction can spend', async () => {
    // 100 held but 96 reserved leaves 4 — the engine reports StalledByReserve
    // in exactly this case, so calling it affordable would promise progress
    // the colony visibly refuses to make.
    const wrapper = await openDialog(
      [option({ id: 'dear', construction_cost: [['structural_metal', 10]] })],
      [stock('structural_metal', 100, 96)],
    )

    await wrapper.get('[data-testid="filter-hide-unaffordable"]').setValue(true)
    expect(wrapper.find('[data-testid="build-card-dear"]').exists()).toBe(false)
  })

  it('hides tech-locked buildings independently of affordability', async () => {
    const wrapper = await openDialog(
      [
        option({ id: 'locked', tech_prerequisite: 'fusion_basics' }),
        option({ id: 'open' }),
      ],
      [],
    )

    await wrapper.get('[data-testid="filter-hide-tech-locked"]').setValue(true)
    expect(wrapper.find('[data-testid="build-card-locked"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="build-card-open"]').exists()).toBe(true)
  })

  it('treats a commodity the colony holds none of as unaffordable', async () => {
    const wrapper = await openDialog(
      [option({ id: 'needs_glass', construction_cost: [['glass', 1]] })],
      [stock('structural_metal', 999)],
    )

    await wrapper.get('[data-testid="filter-hide-unaffordable"]').setValue(true)
    expect(wrapper.find('[data-testid="build-card-needs_glass"]').exists()).toBe(false)
  })

  it('treats a free building as affordable', async () => {
    const wrapper = await openDialog([option({ id: 'free', construction_cost: [] })], [])
    await wrapper.get('[data-testid="filter-hide-unaffordable"]').setValue(true)
    expect(wrapper.find('[data-testid="build-card-free"]').exists()).toBe(true)
  })
})


describe('ColonyView build requirements (issue #423)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    routeParams.colonyId = undefined
    routeParams.buildingType = undefined
    window.localStorage.clear()
  })

  function option(overrides: Record<string, unknown> = {}) {
    return {
      id: 'depot',
      name: 'Depot',
      description: '',
      category: 'Storage',
      slot_cost: 1,
      labor_per_turn: 1,
      construction_turns: 1,
      construction_cost: [] as [string, number][],
      tech_prerequisite: null,
      starter_kit: false,
      max_instances: null,
      ...overrides,
    }
  }

  function stock(commodityId: string, amount: number, reserved = 0) {
    return { commodity_id: commodityId, amount, capacity: null, net_per_turn: 0, reserved }
  }

  async function openDialog(
    catalog: Record<string, unknown>[],
    screenOverrides: Record<string, unknown> = {},
  ) {
    const { listBuildings, getTechTree } = await import('@/services/tauriBridge')
    ;(listBuildings as ReturnType<typeof vi.fn>).mockResolvedValue(catalog)
    ;(getTechTree as ReturnType<typeof vi.fn>).mockResolvedValue(
      screenOverrides.researched ?? [],
    )

    seedColonies()
    const gameStore = useGameStore()
    gameStore.selectedColonyId = 'colony-1'
    gameStore.colonyScreen = colonyScreenWithHqBuilding({
      slot_capacity: 10,
      slots_used: 0,
      stockpile: [],
      construction_queue: [],
      ...screenOverrides,
    })

    const wrapper = mount(ColonyView, {
      global: { stubs: { ...STUBS, ConstructionQueueWindowPanel: false, BuildDialog: false } },
    })
    await flushPromises()
    await wrapper.get('[data-testid="btn-open-build"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="build-dialog"]').exists()).toBe(true)
    return wrapper
  }

  const req = (w: ReturnType<typeof mount>, id: string, key: string) =>
    w.get(`[data-testid="build-req-${id}-${key}"]`)

  it('lists every missing commodity, not just the first', async () => {
    // The gap this closes: `disabledReason` names one blocker, so a building
    // short on two commodities read as having one problem.
    const wrapper = await openDialog(
      [option({ construction_cost: [['structural_metal', 40], ['components', 10]] })],
      { stockpile: [stock('structural_metal', 12)] },
    )

    const metal = req(wrapper, 'depot', 'cost-structural_metal')
    const parts = req(wrapper, 'depot', 'cost-components')
    expect(metal.attributes('data-met')).toBe('false')
    expect(parts.attributes('data-met')).toBe('false')
    expect(metal.text()).toContain('have 12')
    expect(parts.text()).toContain('have 0')
  })

  it('marks a satisfied commodity met, and shows no shortfall for it', async () => {
    const wrapper = await openDialog(
      [option({ construction_cost: [['structural_metal', 40]] })],
      { stockpile: [stock('structural_metal', 40)] },
    )
    const metal = req(wrapper, 'depot', 'cost-structural_metal')
    expect(metal.attributes('data-met')).toBe('true')
    expect(metal.text()).not.toMatch(/have/)
  })

  it('counts reserved stock as unavailable', async () => {
    // Matches what construction can actually spend — the engine reports a
    // distinct StalledByReserve outcome for exactly this case.
    const wrapper = await openDialog(
      [option({ construction_cost: [['structural_metal', 40]] })],
      { stockpile: [stock('structural_metal', 100, 80)] },
    )
    const metal = req(wrapper, 'depot', 'cost-structural_metal')
    expect(metal.attributes('data-met')).toBe('false')
    expect(metal.text()).toContain('have 20')
  })

  it('shows an unmet tech prerequisite', async () => {
    const wrapper = await openDialog([option({ tech_prerequisite: 'automation' })])
    const tech = req(wrapper, 'depot', 'tech-automation')
    expect(tech.attributes('data-met')).toBe('false')
    expect(tech.text()).toContain('automation')
  })

  it('shows a researched tech prerequisite as met', async () => {
    const wrapper = await openDialog([option({ tech_prerequisite: 'automation' })], {
      researched: [{ id: 'automation', name: 'Automation', state: 'researched' }],
    })
    expect(req(wrapper, 'depot', 'tech-automation').attributes('data-met')).toBe('true')
  })

  it('shows slots as a requirement, unmet when the colony is full', async () => {
    const wrapper = await openDialog([option({ slot_cost: 3 })], {
      slot_capacity: 10,
      slots_used: 9,
    })
    const slots = req(wrapper, 'depot', 'slots')
    expect(slots.attributes('data-met')).toBe('false')
    expect(slots.text()).toContain('1 free')
  })

  it('omits the slots requirement for a slot-granting project', async () => {
    // Priced at zero slots by the engine, so "0 slots" would be noise.
    const wrapper = await openDialog([option({ slot_cost: 0 })])
    expect(wrapper.find('[data-testid="build-req-depot-slots"]').exists()).toBe(false)
  })

  it('shows every requirement as met on a fully buildable building', async () => {
    // The decided behaviour: met requirements are always listed, so a card's
    // requirement set reads the same whether or not it happens to be blocked.
    const wrapper = await openDialog(
      [option({ construction_cost: [['structural_metal', 5]], slot_cost: 1 })],
      { stockpile: [stock('structural_metal', 50)] },
    )
    const rows = wrapper.findAll('[data-testid^="build-req-depot-"]')
    expect(rows.length).toBe(2)
    for (const r of rows) expect(r.attributes('data-met')).toBe('true')
  })

  it('conveys status by more than colour — a distinct mark and a spelled-out word', async () => {
    const wrapper = await openDialog(
      [option({ construction_cost: [['structural_metal', 5], ['components', 5]] })],
      { stockpile: [stock('structural_metal', 50)] },
    )
    const met = req(wrapper, 'depot', 'cost-structural_metal')
    const unmet = req(wrapper, 'depot', 'cost-components')

    expect(met.get('.req-mark').text()).not.toBe(unmet.get('.req-mark').text())
    expect(met.get('.req-sr').text()).toBe('met:')
    expect(unmet.get('.req-sr').text()).toBe('missing:')
  })
})


describe('ColonyView site requirements (issue #410)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    routeParams.colonyId = undefined
    routeParams.buildingType = undefined
    window.localStorage.clear()
  })

  function option(overrides: Record<string, unknown> = {}) {
    return {
      id: 'wave_plant',
      name: 'Wave Plant',
      description: '',
      category: 'Power',
      slot_cost: 1,
      labor_per_turn: 1,
      construction_turns: 1,
      construction_cost: [] as [string, number][],
      tech_prerequisite: null,
      starter_kit: false,
      max_instances: null,
      ...overrides,
    }
  }

  async function openDialog(
    catalog: Record<string, unknown>[],
    siteRequirements: { building_type: string; label: string; met: boolean }[],
  ) {
    const { listBuildings } = await import('@/services/tauriBridge')
    ;(listBuildings as ReturnType<typeof vi.fn>).mockResolvedValue(catalog)

    seedColonies()
    const gameStore = useGameStore()
    gameStore.selectedColonyId = 'colony-1'
    gameStore.colonyScreen = colonyScreenWithHqBuilding({
      slot_capacity: 10,
      slots_used: 0,
      stockpile: [],
      construction_queue: [],
      site_requirements: siteRequirements,
    })

    const wrapper = mount(ColonyView, {
      global: { stubs: { ...STUBS, ConstructionQueueWindowPanel: false, BuildDialog: false } },
    })
    await flushPromises()
    await wrapper.get('[data-testid="btn-open-build"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="build-dialog"]').exists()).toBe(true)
    return wrapper
  }

  it('shows an unmet site condition as a failed requirement and disables Queue', async () => {
    const wrapper = await openDialog(
      [option()],
      [{ building_type: 'wave_plant', label: 'ocean within 2 hexes', met: false }],
    )

    const req = wrapper.get('[data-testid="build-req-wave_plant-site-0"]')
    expect(req.attributes('data-met')).toBe('false')
    expect(req.text()).toContain('ocean within 2 hexes')
    expect(wrapper.get('[data-testid="btn-queue-wave_plant"]').attributes('disabled')).toBeDefined()
  })

  it('shows a satisfied site condition as met, and leaves the building buildable', async () => {
    const wrapper = await openDialog(
      [option()],
      [{ building_type: 'wave_plant', label: 'ocean within 2 hexes', met: true }],
    )

    expect(wrapper.get('[data-testid="build-req-wave_plant-site-0"]').attributes('data-met')).toBe(
      'true',
    )
    expect(
      wrapper.get('[data-testid="btn-queue-wave_plant"]').attributes('disabled'),
    ).toBeUndefined()
  })

  it('lists every unmet site condition, not just the first', async () => {
    const wrapper = await openDialog(
      [option()],
      [
        { building_type: 'wave_plant', label: 'ocean within 2 hexes', met: false },
        { building_type: 'wave_plant', label: 'thin atmosphere or denser', met: false },
      ],
    )

    expect(wrapper.get('[data-testid="build-req-wave_plant-site-0"]').attributes('data-met')).toBe('false')
    expect(wrapper.get('[data-testid="build-req-wave_plant-site-1"]').attributes('data-met')).toBe('false')
    // ...and the disabled reason names both rather than stopping at one.
    const title = wrapper.get('[data-testid="build-card-wave_plant"]').attributes('title') ?? ''
    expect(title).toContain('ocean within 2 hexes')
    expect(title).toContain('thin atmosphere or denser')
  })

  it('does not attach another building\'s site conditions', async () => {
    const wrapper = await openDialog(
      [option(), option({ id: 'solar', name: 'Solar' })],
      [{ building_type: 'wave_plant', label: 'ocean within 2 hexes', met: false }],
    )

    expect(wrapper.find('[data-testid="build-req-solar-site-0"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="btn-queue-solar"]').attributes('disabled')).toBeUndefined()
  })

  it('leaves buildings alone when the colony screen carries no site data', async () => {
    // Older payloads (and any colony screen predating #410) omit the field.
    const wrapper = await openDialog([option()], [])
    expect(wrapper.find('[data-testid="build-req-wave_plant-site-0"]').exists()).toBe(false)
    expect(
      wrapper.get('[data-testid="btn-queue-wave_plant"]').attributes('disabled'),
    ).toBeUndefined()
  })
})
