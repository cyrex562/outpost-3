import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
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
