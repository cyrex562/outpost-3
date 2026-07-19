import { describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'
import type { BuildingDetail } from '@/services/tauriBridge'

const getBuildingDetail = vi.fn<[string, string], Promise<BuildingDetail>>()
const setActiveRecipe = vi.fn<[string, string, string], Promise<unknown>>()

vi.mock('@/services/tauriBridge', () => ({
  getBuildingDetail: (colonyId: string, buildingType: string) => getBuildingDetail(colonyId, buildingType),
  setActiveRecipe: (colonyId: string, buildingType: string, recipeId: string) =>
    setActiveRecipe(colonyId, buildingType, recipeId),
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
    available_recipes: [],
    concurrent_recipes: [],
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

  it('shows no recipe selector when there is only one recipe', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail({}))
    const wrapper = mount(BuildingDetailsHud, {
      props: { colonyId: 'colony-1', buildingType: 'research_lab' },
    })
    await flushPromises()
    expect(wrapper.find('[data-testid="recipe-selector"]').exists()).toBe(false)
  })

  it('shows a recipe selector and switches the active recipe (#166)', async () => {
    const recipeA = {
      recipe_id: 'refine_ore_to_plate',
      name: 'Refine Ore to Plate',
      inputs: [{ commodity_id: 'structural_ore', quantity: 5 }],
      outputs: [{ commodity_id: 'structural_metal', quantity: 4 }],
      cycle_sols: 1,
    }
    const recipeB = {
      recipe_id: 'refine_plate_to_components',
      name: 'Refine Plate to Components',
      inputs: [{ commodity_id: 'structural_metal', quantity: 3 }],
      outputs: [{ commodity_id: 'components', quantity: 2 }],
      cycle_sols: 2,
    }
    getBuildingDetail.mockResolvedValueOnce(
      makeDetail({
        building_type: 'refinery',
        name: 'Refinery',
        recipe: recipeA,
        available_recipes: [recipeA, recipeB],
      }),
    )
    const wrapper = mount(BuildingDetailsHud, {
      props: { colonyId: 'colony-1', buildingType: 'refinery' },
    })
    await flushPromises()

    const select = wrapper.find('[data-testid="recipe-select"]')
    expect(select.exists()).toBe(true)
    const options = select.findAll('option').map((o) => o.attributes('value'))
    expect(options).toEqual(['refine_ore_to_plate', 'refine_plate_to_components'])

    getBuildingDetail.mockResolvedValueOnce(
      makeDetail({
        building_type: 'refinery',
        name: 'Refinery',
        recipe: recipeB,
        available_recipes: [recipeA, recipeB],
      }),
    )
    setActiveRecipe.mockResolvedValueOnce([])
    await select.setValue('refine_plate_to_components')
    await flushPromises()

    expect(setActiveRecipe).toHaveBeenCalledWith('colony-1', 'refinery', 'refine_plate_to_components')
    expect(wrapper.text()).toContain('Refine Plate to Components')
  })

  it('renders always-on recipes for a concurrent-only building instead of "No recipe" (colony_hq, issue #272)', async () => {
    getBuildingDetail.mockResolvedValueOnce(
      makeDetail({
        building_type: 'colony_hq',
        name: 'Colony HQ',
        recipe: null,
        available_recipes: [],
        concurrent_recipes: [
          {
            recipe_id: 'hq_generate_power',
            name: 'Generate Power (Colony HQ)',
            inputs: [],
            outputs: [{ commodity_id: 'power', quantity: 24 }],
            cycle_sols: 1,
          },
          {
            recipe_id: 'hq_pump_water',
            name: 'Pump Water (Colony HQ)',
            inputs: [],
            outputs: [{ commodity_id: 'water', quantity: 24 }],
            cycle_sols: 1,
          },
        ],
      }),
    )
    const wrapper = mount(BuildingDetailsHud, {
      props: { colonyId: 'colony-1', buildingType: 'colony_hq' },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="concurrent-recipes-section"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Generate Power (Colony HQ)')
    expect(wrapper.text()).toContain('Pump Water (Colony HQ)')
    expect(wrapper.text()).not.toContain('No recipe (storage/habitat building).')
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
