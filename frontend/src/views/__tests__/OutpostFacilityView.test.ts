import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { reactive } from 'vue'
import OutpostFacilityView from '@/views/OutpostFacilityView.vue'
import type { BuildingDetail } from '@/services/tauriBridge'

const routerPush = vi.fn()
const routeParams = reactive<{ outpostId: string; buildingType: string }>({
  outpostId: 'outpost-1',
  buildingType: 'mining_outpost',
})

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => ({ params: routeParams }),
}))

const getOutpostBuildingDetail = vi.fn<[string, string], Promise<BuildingDetail>>()

vi.mock('@/services/tauriBridge', () => ({
  getOutpostBuildingDetail: (outpostId: string, buildingType: string) =>
    getOutpostBuildingDetail(outpostId, buildingType),
  setOutpostActiveRecipe: vi.fn(),
}))

function makeDetail(): BuildingDetail {
  return {
    building_type: 'mining_outpost',
    name: 'Mining Outpost',
    description: 'Extracts ore.',
    category: 'Production',
    slot_cost: 1,
    power_delta: 0,
    maintenance: [],
    recipe: null,
    available_recipes: [],
    concurrent_recipes: [],
    lines: [],
    last_run: null,
  }
}

describe('OutpostFacilityView (navigation rework #7 phase 4: routed outpost facility page)', () => {
  beforeEach(() => {
    routerPush.mockReset()
    getOutpostBuildingDetail.mockReset()
    routeParams.outpostId = 'outpost-1'
    routeParams.buildingType = 'mining_outpost'
  })

  it('fetches outpost-scoped detail for the outpost/building named by the route params', async () => {
    getOutpostBuildingDetail.mockResolvedValueOnce(makeDetail())
    const wrapper = mount(OutpostFacilityView)
    await new Promise((r) => setTimeout(r, 0))

    expect(getOutpostBuildingDetail).toHaveBeenCalledWith('outpost-1', 'mining_outpost')
    expect(wrapper.find('[data-testid="facility-page"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Mining Outpost')
  })

  it('navigates back to the outpost route when closed', async () => {
    getOutpostBuildingDetail.mockResolvedValueOnce(makeDetail())
    const wrapper = mount(OutpostFacilityView)
    await new Promise((r) => setTimeout(r, 0))

    await wrapper.get('[data-testid="facility-back"]').trigger('click')

    expect(routerPush).toHaveBeenCalledWith({ name: 'outpost', params: { outpostId: 'outpost-1' } })
  })
})
