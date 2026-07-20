import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { reactive } from 'vue'
import FacilityView from '@/views/FacilityView.vue'
import type { BuildingDetail } from '@/services/tauriBridge'

const routerPush = vi.fn()
const routeParams = reactive<{ colonyId: string; buildingType: string }>({
  colonyId: 'colony-1',
  buildingType: 'colony_hq',
})

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => ({ params: routeParams }),
}))

const getBuildingDetail = vi.fn<[string, string], Promise<BuildingDetail>>()

vi.mock('@/services/tauriBridge', () => ({
  getBuildingDetail: (colonyId: string, buildingType: string) => getBuildingDetail(colonyId, buildingType),
  setActiveRecipe: vi.fn(),
}))

function makeDetail(): BuildingDetail {
  return {
    building_type: 'colony_hq',
    name: 'Colony HQ',
    description: 'The physical plant.',
    category: 'Services',
    slot_cost: 1,
    power_delta: -13,
    maintenance: [],
    recipe: null,
    available_recipes: [],
    concurrent_recipes: [],
    last_run: null,
  }
}

describe('FacilityView (navigation rework #7 phase 2: routed facility page)', () => {
  beforeEach(() => {
    routerPush.mockReset()
    getBuildingDetail.mockReset()
    routeParams.colonyId = 'colony-1'
    routeParams.buildingType = 'colony_hq'
  })

  it('fetches detail for the colony/building named by the route params', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail())
    const wrapper = mount(FacilityView)
    await new Promise((r) => setTimeout(r, 0))

    expect(getBuildingDetail).toHaveBeenCalledWith('colony-1', 'colony_hq')
    expect(wrapper.find('[data-testid="facility-page"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Colony HQ')
  })

  it('renders as a page, not a modal backdrop', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail())
    const wrapper = mount(FacilityView)
    await new Promise((r) => setTimeout(r, 0))

    expect(wrapper.find('[data-testid="building-details-hud"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="facility-page"]').exists()).toBe(true)
  })

  it('navigates back to the colony route when closed', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail())
    const wrapper = mount(FacilityView)
    await new Promise((r) => setTimeout(r, 0))

    await wrapper.get('[data-testid="facility-back"]').trigger('click')

    expect(routerPush).toHaveBeenCalledWith({ name: 'colony', params: { colonyId: 'colony-1' } })
  })
})
