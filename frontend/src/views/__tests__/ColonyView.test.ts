import { afterEach, describe, expect, it, vi, beforeEach } from 'vitest'
import { enableAutoUnmount, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent, h, nextTick, onMounted, reactive } from 'vue'
import ColonyView from '@/views/ColonyView.vue'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'

// Issue #322 added a route-reactive `watch` in `ColonyView` (opening the
// building-details panel on a deep link) with a real side effect — unlike
// this suite's earlier watchers, an un-unmounted instance from a prior test
// keeps reacting to the shared `routeParams` object and leaks `addPanel`
// calls into later tests. Auto-unmounting after each test keeps instances
// from earlier tests from observing later mutations of `routeParams`.
enableAutoUnmount(afterEach)

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
 * Real `dockview-vue` needs `ResizeObserver` and real layout measurement,
 * neither of which jsdom provides — matching this issue's own testing note
 * ("most drag-to-dock behaviour isn't assertable in jsdom"). This stub
 * renders every registered panel component directly (ignoring dockview's
 * actual grid/tab/position bookkeeping, which `colonyDock.test.ts` covers
 * with a plain mock `addPanel` recorder instead) and fires `ready` with a
 * minimal fake `DockviewApi`, which is enough for `ColonyView.vue`'s own
 * behaviour (opening the build dialog, floating the building-details
 * window, etc.) to be exercised end-to-end.
 */
/** Recorder for the fake `DockviewApi`'s `addPanel` calls, and the fake
 * panel handles it hands back — stand-ins for real dockview's own panel
 * bookkeeping (see `dockApi` below), asserted on directly since the stub
 * never actually renders per-panel `params`. `getPanel` looks panels up from
 * what `addPanel` recorded, so retargeting an already-open panel (issue
 * #322) is exercised the same way it is against a real `DockviewApi`. */
const addPanelCalls: { id: string; component: string; params?: unknown }[] = []
const addedPanelHandles: {
  api: { updateParameters: ReturnType<typeof vi.fn>; setTitle: ReturnType<typeof vi.fn>; setActive: ReturnType<typeof vi.fn> }
}[] = []

const DockviewVueStub = defineComponent({
  name: 'dockview',
  props: { components: { type: Object, default: () => ({}) } },
  emits: ['ready'],
  setup(props, { emit }) {
    const panelsById = new Map<string, (typeof addedPanelHandles)[number]>()
    const fakeApi = {
      addPanel: vi.fn((opts: { id: string; component: string; params?: unknown }) => {
        addPanelCalls.push(opts)
        const handle = { api: { updateParameters: vi.fn(), setTitle: vi.fn(), setActive: vi.fn() } }
        addedPanelHandles.push(handle)
        panelsById.set(opts.id, handle)
        return handle
      }),
      getPanel: vi.fn((id: string) => panelsById.get(id)),
      onDidLayoutChange: () => ({ dispose: () => {} }),
      fromJSON: vi.fn(),
      toJSON: () => ({}),
      clear: vi.fn(),
    }
    onMounted(() => emit('ready', { api: fakeApi }))
    return () =>
      h(
        'div',
        { 'data-testid': 'colony-dockview-stub' },
        Object.values(props.components ?? {}).map((comp) => h(comp as never)),
      )
  },
})

const STUBS = {
  dockview: DockviewVueStub,
  DockVitalStatsPanel: true,
  DockUtilitiesPanel: true,
  DockCommoditiesPanel: true,
  DockBuildingsPanel: true,
  DockConstructionQueuePanel: true,
  DockAlertsPanel: true,
  DockBuildingDetailsPanel: true,
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

describe('ColonyView colony selection (navigation rework #7 phase 1: route param is source of truth)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    routerReplace.mockReset()
    routeParams.colonyId = undefined
    routeParams.buildingType = undefined
    addPanelCalls.length = 0
    addedPanelHandles.length = 0
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

  it('renders the dock layout and a reset-layout button (issue #321)', () => {
    seedColonies()
    routeParams.colonyId = 'colony-1'

    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })

    expect(wrapper.find('[data-testid="colony-dockview"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="btn-reset-layout"]').exists()).toBe(true)
  })

  it('opens the building-details dock panel (not a route navigation) when a building requests details (issue #322)', async () => {
    seedColonies()
    routeParams.colonyId = 'colony-1'
    const gameStore = useGameStore()
    gameStore.colonyScreen = {
      colony_id: 'colony-1',
      name: 'Alpha Base',
      population: 100,
      stability: 0.9,
      slots_used: 1,
      slot_capacity: 5,
      labour_available: 5,
      labour_total: 10,
      labour_demanded: 4,
      labour_employed: 4,
      labour_unemployed: 1,
      resources: [
        { resource_id: 'power', name: 'Power', amount: 24, kind: 'flow', unit: 'MW' },
      ],
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
          running_recipe_ids: ['hq_generate_power', 'hq_pump_water'],
          inputs: [],
          outputs: [
            { commodity_id: 'power', quantity: 24 },
            { commodity_id: 'water', quantity: 24 },
          ],
        },
      ],
      stockpile: [],
      construction_queue: [],
      manual_override: false,
    }

    // Mount with the real BuildingsPanel and its Dock wrapper (not stubbed)
    // so its actual "view details" button emits the real event ColonyView's
    // dock context listens for — everything downstream of that (dockview
    // itself) is the fake api above, asserted on directly.
    const wrapper = mount(ColonyView, {
      global: { stubs: { ...STUBS, DockBuildingsPanel: false } },
    })
    // The default 6-panel layout is built on mount — filter those out so
    // only the building-details panel's own addPanel calls are asserted on.
    const detailsPanelCalls = () => addPanelCalls.filter((c) => c.id === 'building-details')
    expect(detailsPanelCalls()).toHaveLength(0)

    await wrapper.get('[data-testid="view-details-colony_hq"]').trigger('click')

    expect(routerPush).not.toHaveBeenCalledWith(expect.objectContaining({ name: 'facility' }))
    expect(detailsPanelCalls()).toHaveLength(1)
    expect(detailsPanelCalls()[0]).toMatchObject({
      id: 'building-details',
      component: 'building-details',
      params: { buildingType: 'colony_hq' },
    })
  })

  it('retargets the existing building-details panel instead of opening a second one', async () => {
    seedColonies()
    routeParams.colonyId = 'colony-1'
    const gameStore = useGameStore()
    gameStore.colonyScreen = {
      colony_id: 'colony-1',
      name: 'Alpha Base',
      population: 100,
      stability: 0.9,
      slots_used: 2,
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
      stockpile: [],
      construction_queue: [],
      manual_override: false,
    }

    const wrapper = mount(ColonyView, {
      global: { stubs: { ...STUBS, DockBuildingsPanel: false } },
    })
    const detailsPanelCalls = () => addPanelCalls.filter((c) => c.id === 'building-details')

    await wrapper.get('[data-testid="view-details-colony_hq"]').trigger('click')
    expect(detailsPanelCalls()).toHaveLength(1)

    await wrapper.get('[data-testid="view-details-research_lab"]').trigger('click')

    expect(detailsPanelCalls()).toHaveLength(1)
    const panelIndex = addPanelCalls.findIndex((c) => c.id === 'building-details')
    const panel = addedPanelHandles[panelIndex]
    expect(panel?.api.updateParameters).toHaveBeenCalledWith({ buildingType: 'research_lab' })
    expect(panel?.api.setActive).toHaveBeenCalled()
  })

  it('opens the building-details panel for a deep-linked /colony/:colonyId/facility/:buildingType route', async () => {
    seedColonies()
    routeParams.colonyId = 'colony-1'
    routeParams.buildingType = 'colony_hq'

    mount(ColonyView, { global: { stubs: STUBS } })
    await nextTick()

    const detailsPanelCalls = addPanelCalls.filter((c) => c.id === 'building-details')
    expect(detailsPanelCalls).toHaveLength(1)
    expect(detailsPanelCalls[0]).toMatchObject({
      id: 'building-details',
      component: 'building-details',
      params: { buildingType: 'colony_hq' },
    })
  })
})
