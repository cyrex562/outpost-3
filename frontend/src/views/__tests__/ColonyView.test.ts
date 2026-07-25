import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { reactive } from 'vue'
import ColonyView from '@/views/ColonyView.vue'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'

const routerPush = vi.fn()
const routerReplace = vi.fn()
const routeParams = reactive<{ colonyId: string | undefined }>({ colonyId: undefined })

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush, replace: routerReplace }),
  useRoute: () => ({ params: routeParams }),
}))

vi.mock('@/services/tauriBridge', () => ({
  isTauri: false,
  listBuildings: vi.fn().mockResolvedValue([]),
  getTechTree: vi.fn().mockResolvedValue([]),
}))

const STUBS = {
  VitalStatsPanel: true,
  CommoditiesPanel: true,
  BuildingsPanel: true,
  ConstructionQueuePanel: true,
  BuildDialog: true,
  AlertsPanel: true,
  Splitpanes: true,
  Pane: true,
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

  it('navigates to the facility route when a building requests details (navigation rework #7 phase 2)', async () => {
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
      buildings: [
        {
          building_type: 'colony_hq',
          labour_assigned: 0,
          slot_cost: 1,
          full_capacity: true,
          scale: 1.0,
          shortfall_reason: null,
          always_on: true,
        },
      ],
      stockpile: [],
      construction_queue: [],
      manual_override: false,
    }

    // Mount with the real BuildingsPanel (not stubbed) so its actual
    // "view details" button emits the real event ColonyView listens for —
    // Splitpanes/Pane need slot-rendering stubs instead of the default
    // auto-stub (which drops slot content) so BuildingsPanel actually mounts.
    const wrapper = mount(ColonyView, {
      global: {
        stubs: {
          ...STUBS,
          BuildingsPanel: false,
          Splitpanes: { template: '<div><slot /></div>' },
          Pane: { template: '<div><slot /></div>' },
        },
      },
    })
    await wrapper.get('[data-testid="view-details-colony_hq"]').trigger('click')

    expect(routerPush).toHaveBeenCalledWith({
      name: 'facility',
      params: { colonyId: 'colony-1', buildingType: 'colony_hq' },
    })
  })

})
