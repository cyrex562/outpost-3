import { describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'
import type { BuildingDetail } from '@/services/tauriBridge'

const getBuildingDetail = vi.fn<[string, string], Promise<BuildingDetail>>()

vi.mock('@/services/tauriBridge', () => ({
  getBuildingDetail: (colonyId: string, buildingType: string) => getBuildingDetail(colonyId, buildingType),
}))

function makeDetail(overrides: Partial<BuildingDetail>): BuildingDetail {
  return {
    building_type: 'research_lab',
    name: 'Research Lab',
    description: 'Generates research points.',
    category: 'Research',
    slot_cost: 1,
    power_delta: 5,
    maintenance: [{ commodity_id: 'spare_parts', quantity: 0.5 }],
    recipe: {
      recipe_id: 'research_recipe',
      name: 'Basic Research',
      inputs: [{ commodity_id: 'water', quantity: 1 }],
      outputs: [{ commodity_id: 'research', quantity: 2 }],
      cycle_sols: 1,
    },
    last_run: {
      scale: 1.0,
      is_full_production: true,
      shortfalls: [],
    },
    ...overrides,
  }
}

describe('BuildingDetailsHud (#182)', () => {
  it('is hidden when buildingType is null', () => {
    const wrapper = mount(BuildingDetailsHud, {
      props: { colonyId: 'colony-1', buildingType: null },
    })
    expect(wrapper.find('[data-testid="building-details-hud"]').exists()).toBe(false)
  })

  it('fetches and renders detail when a building type is selected', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail({}))
    const wrapper = mount(BuildingDetailsHud, {
      props: { colonyId: 'colony-1', buildingType: 'research_lab' },
    })
    await flushPromises()

    expect(getBuildingDetail).toHaveBeenCalledWith('colony-1', 'research_lab')
    expect(wrapper.text()).toContain('Research Lab')
    expect(wrapper.text()).toContain('Basic Research')
    expect(wrapper.find('[data-testid="maintenance-short-indicator"]').exists()).toBe(false)
  })

  it('shows the MAINTENANCE SHORT indicator when a maintenance_short shortfall is present', async () => {
    getBuildingDetail.mockResolvedValueOnce(
      makeDetail({
        last_run: {
          scale: 0.4,
          is_full_production: false,
          shortfalls: [{ kind: 'maintenance_short', commodity_id: 'spare_parts', effective_scale: 0.4 }],
        },
      }),
    )
    const wrapper = mount(BuildingDetailsHud, {
      props: { colonyId: 'colony-1', buildingType: 'research_lab' },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="maintenance-short-indicator"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="shortfall-maintenance_short"]').exists()).toBe(true)
  })

  it('emits close when the backdrop is clicked', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail({}))
    const wrapper = mount(BuildingDetailsHud, {
      props: { colonyId: 'colony-1', buildingType: 'research_lab' },
    })
    await flushPromises()

    await wrapper.find('[data-testid="close-details-hud"]').trigger('click')
    expect(wrapper.emitted('close')).toBeTruthy()
  })
})
