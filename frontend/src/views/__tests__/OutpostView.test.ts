import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { reactive } from 'vue'
import OutpostView from '@/views/OutpostView.vue'
import { useWorldStore } from '@/stores/worldStore'
import type { Outpost } from '@/services/tauriBridge'

const routerPush = vi.fn()
const routeParams = reactive<{ outpostId: string | undefined }>({ outpostId: 'outpost-1' })

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => ({ params: routeParams }),
}))

const listOutposts = vi.fn<[], Promise<Outpost[]>>()

vi.mock('@/services/tauriBridge', () => ({
  isTauri: false,
  listOutposts: () => listOutposts(),
}))

function makeOutpost(overrides: Partial<Outpost>): Outpost {
  return {
    id: 'outpost-1',
    name: 'Forward Base',
    parent_colony_id: 'colony-1',
    body_id: 'body-1',
    body_name: 'Luna',
    slot_capacity: 3,
    slots_used: 1,
    buildings: ['excavation_rig'],
    pool: [],
    ...overrides,
  }
}

describe('OutpostView (navigation rework #7 phase 4: outpost drill-down page)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    listOutposts.mockReset()
    routerPush.mockReset()
    routeParams.outpostId = 'outpost-1'
  })

  it('renders the outpost named by the route param, filtered from the full list', async () => {
    listOutposts.mockResolvedValueOnce([
      makeOutpost({ id: 'outpost-1', name: 'Forward Base' }),
      makeOutpost({ id: 'outpost-2', name: 'Rear Base' }),
    ])
    const wrapper = mount(OutpostView)
    await flushPromises()

    expect(wrapper.text()).toContain('Forward Base')
    expect(wrapper.text()).not.toContain('Rear Base')
  })

  it('resolves the owning colony name from worldStore', async () => {
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
    }
    listOutposts.mockResolvedValueOnce([makeOutpost({ parent_colony_id: 'colony-1' })])
    const wrapper = mount(OutpostView)
    await flushPromises()

    expect(wrapper.text()).toContain('Alpha Base')
  })

  it('shows a not-found state when the route id matches no outpost', async () => {
    listOutposts.mockResolvedValueOnce([makeOutpost({ id: 'outpost-2' })])
    const wrapper = mount(OutpostView)
    await flushPromises()

    expect(wrapper.find('[data-testid="outpost-not-found"]').exists()).toBe(true)
  })

  it('navigates to the outpost facility page when a building is clicked', async () => {
    listOutposts.mockResolvedValueOnce([makeOutpost({ id: 'outpost-1', buildings: ['mining_outpost'] })])
    const wrapper = mount(OutpostView)
    await flushPromises()

    await wrapper.get('[data-testid="outpost-building-mining_outpost-0"]').trigger('click')

    expect(routerPush).toHaveBeenCalledWith({
      name: 'outpost-facility',
      params: { outpostId: 'outpost-1', buildingType: 'mining_outpost' },
    })
  })

  it('shows the empty state when the outpost has no buildings', async () => {
    listOutposts.mockResolvedValueOnce([makeOutpost({ id: 'outpost-1', buildings: [] })])
    const wrapper = mount(OutpostView)
    await flushPromises()

    expect(wrapper.text()).toContain('No buildings constructed yet.')
  })
})
