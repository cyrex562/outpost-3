import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import BuildingsListView from '@/views/BuildingsListView.vue'
import { useWorldStore } from '@/stores/worldStore'

const routerPush = vi.fn()

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
}))

function seedColonies(): void {
  const worldStore = useWorldStore()
  worldStore.world.colonies = {
    'colony-1': {
      id: 'colony-1',
      name: 'Alpha Base',
      population: 100,
      stability: 0.9,
      available_labour: 5,
      buildings: ['colony_hq', 'excavation_rig'],
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
      buildings: ['colony_hq'],
      active_projects: [],
      commodity_pool: [],
      active_construction: [],
    },
  }
}

describe('BuildingsListView (navigation rework #7 phase 3: all-buildings list)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
  })

  it('flattens every colony\'s buildings into one table, sorted by colony then building type', () => {
    seedColonies()
    const wrapper = mount(BuildingsListView)

    const rows = wrapper.findAll('tbody tr')
    expect(rows).toHaveLength(3)
    expect(wrapper.text()).toContain('3 buildings across 2 colonies')
    // Alpha Base < Beta Outpost, and within Alpha Base colony_hq < excavation_rig.
    expect(rows.map((r) => r.find('td').text())).toEqual(['colony_hq', 'excavation_rig', 'colony_hq'])
  })

  it('navigates to the facility page when a row is clicked', async () => {
    seedColonies()
    const wrapper = mount(BuildingsListView)

    await wrapper.get('[data-testid="building-row-colony-1-colony_hq-0"]').trigger('click')

    expect(routerPush).toHaveBeenCalledWith({
      name: 'facility',
      params: { colonyId: 'colony-1', buildingType: 'colony_hq' },
    })
  })

  it('shows the empty state when no colonies have any buildings', () => {
    const wrapper = mount(BuildingsListView)
    expect(wrapper.text()).toContain('No buildings constructed anywhere yet.')
  })
})
