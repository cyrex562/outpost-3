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
  CommandPanel: true,
  PopulationPanel: true,
  CommoditiesPanel: true,
  BuildingsPanel: true,
  BuildingDetailsHud: true,
  ConstructionQueuePanel: true,
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
    expect(wrapper.find('.tab.active').text()).toBe('Beta Outpost')
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

  it('navigates via router.push when a colony tab is clicked', async () => {
    seedColonies()
    routeParams.colonyId = 'colony-1'

    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })
    const tabs = wrapper.findAll('.tab')
    const betaTab = tabs.find((t) => t.text() === 'Beta Outpost')
    await betaTab?.trigger('click')

    expect(routerPush).toHaveBeenCalledWith({ name: 'colony', params: { colonyId: 'colony-2' } })
  })

  it('does not re-push when clicking the already-active tab (avoids a duplicate-navigation warning)', async () => {
    seedColonies()
    routeParams.colonyId = 'colony-1'

    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })
    const tabs = wrapper.findAll('.tab')
    const alphaTab = tabs.find((t) => t.text() === 'Alpha Base')
    await alphaTab?.trigger('click')

    expect(routerPush).not.toHaveBeenCalled()
  })

  it('shows the empty state when no colonies exist', () => {
    const wrapper = mount(ColonyView, { global: { stubs: STUBS } })
    expect(wrapper.find('[data-testid="no-colonies"]').exists()).toBe(true)
  })
})
