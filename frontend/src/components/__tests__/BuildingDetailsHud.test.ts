import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'
import type { BuildingDetail } from '@/services/tauriBridge'

const getBuildingDetail = vi.fn<[string, string], Promise<BuildingDetail>>()
const setActiveRecipe = vi.fn<[string, string, string], Promise<unknown>>()
const getOutpostBuildingDetail = vi.fn<[string, string], Promise<BuildingDetail>>()
const setOutpostActiveRecipe = vi.fn<[string, string, string], Promise<unknown>>()

vi.mock('@/services/tauriBridge', () => ({
  getBuildingDetail: (colonyId: string, buildingType: string) => getBuildingDetail(colonyId, buildingType),
  setActiveRecipe: (colonyId: string, buildingType: string, recipeId: string) =>
    setActiveRecipe(colonyId, buildingType, recipeId),
  getOutpostBuildingDetail: (outpostId: string, buildingType: string) =>
    getOutpostBuildingDetail(outpostId, buildingType),
  setOutpostActiveRecipe: (outpostId: string, buildingType: string, recipeId: string) =>
    setOutpostActiveRecipe(outpostId, buildingType, recipeId),
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
    lines: [],
    last_run: {
      scale: 1.0,
      is_full_production: true,
      shortfalls: [],
    },
    ...overrides,
  }
}

describe('BuildingDetailsHud (#182)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('is hidden when buildingType is null', () => {
    const wrapper = mount(BuildingDetailsHud, {
      props: { ownerId: 'colony-1', buildingType: null },
    })
    expect(wrapper.find('[data-testid="building-details-hud"]').exists()).toBe(false)
  })

  it('fetches and renders detail when a building type is selected', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail({}))
    const wrapper = mount(BuildingDetailsHud, {
      props: { ownerId: 'colony-1', buildingType: 'research_lab' },
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
          shortfalls: [{ kind: 'maintenance_short', commodity_id: 'spare_parts', effective_scale: 0.4, deficit: 0.3 }],
        },
      }),
    )
    const wrapper = mount(BuildingDetailsHud, {
      props: { ownerId: 'colony-1', buildingType: 'research_lab' },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="maintenance-short-indicator"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="shortfall-maintenance_short"]').exists()).toBe(true)
  })

  it('shows no recipe selector when there is only one recipe', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail({}))
    const wrapper = mount(BuildingDetailsHud, {
      props: { ownerId: 'colony-1', buildingType: 'research_lab' },
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
      props: { ownerId: 'colony-1', buildingType: 'refinery' },
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
        lines: [],
      }),
    )
    const wrapper = mount(BuildingDetailsHud, {
      props: { ownerId: 'colony-1', buildingType: 'colony_hq' },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="concurrent-recipes-section"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Generate Power (Colony HQ)')
    expect(wrapper.text()).toContain('Pump Water (Colony HQ)')
    expect(wrapper.text()).not.toContain('No recipe (storage/habitat building).')
  })

  describe('production lines (#272)', () => {
    const smelt = {
      recipe_id: 'foundry_smelt_ore',
      name: 'Smelt Structural Ore',
      inputs: [{ commodity_id: 'structural_ore', quantity: 5 }],
      outputs: [{ commodity_id: 'structural_metal', quantity: 2.5 }],
      cycle_sols: 1,
    }
    const smeltConductive = {
      recipe_id: 'foundry_smelt_conductive',
      name: 'Smelt Conductive Ore',
      inputs: [{ commodity_id: 'conductive_ore', quantity: 5 }],
      outputs: [{ commodity_id: 'conductive_metal', quantity: 2 }],
      cycle_sols: 1,
    }
    const fabricate = {
      recipe_id: 'machine_shop_fabricate_components',
      name: 'Fabricate Components',
      inputs: [{ commodity_id: 'structural_metal', quantity: 2 }],
      outputs: [{ commodity_id: 'components', quantity: 1 }],
      cycle_sols: 1,
    }

    /** A two-line `fabrication_complex`: foundry (pick-one) + machine shop. */
    function twoLineDetail() {
      return makeDetail({
        building_type: 'fabrication_complex',
        name: 'Fabrication Complex',
        recipe: smelt,
        available_recipes: [smelt, smeltConductive],
        concurrent_recipes: [],
        lines: [
          {
            line: 'foundry',
            always_on: false,
            selected_recipe_id: 'foundry_smelt_ore',
            alternatives: [smelt, smeltConductive],
          },
          {
            line: 'machine_shop',
            always_on: true,
            selected_recipe_id: 'machine_shop_fabricate_components',
            alternatives: [fabricate],
          },
        ],
      })
    }

    it('renders one section per line, each showing its own selected recipe flows', async () => {
      getBuildingDetail.mockResolvedValueOnce(twoLineDetail())
      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerId: 'colony-1', buildingType: 'fabrication_complex' },
      })
      await flushPromises()

      const foundry = wrapper.find('[data-testid="line-section-foundry"]')
      const shop = wrapper.find('[data-testid="line-section-machine_shop"]')
      expect(foundry.exists()).toBe(true)
      expect(shop.exists()).toBe(true)

      // Each line shows only its own selected recipe's flows.
      expect(foundry.text()).toContain('structural_metal ×2.5')
      expect(foundry.text()).not.toContain('components ×1')
      expect(shop.text()).toContain('components ×1')

      // The unselected foundry alternative is an option, not a rendered flow.
      expect(foundry.text()).not.toContain('conductive_metal ×2')

      // The flat pre-#272 picker must be gone — it presented simultaneous
      // lines as mutually exclusive alternatives.
      expect(wrapper.find('[data-testid="recipe-section"]').exists()).toBe(false)
      expect(wrapper.find('[data-testid="recipe-select"]').exists()).toBe(false)
    })

    it('offers a selector only on the line that has alternatives, and marks always-on lines', async () => {
      getBuildingDetail.mockResolvedValueOnce(twoLineDetail())
      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerId: 'colony-1', buildingType: 'fabrication_complex' },
      })
      await flushPromises()

      expect(wrapper.find('[data-testid="line-select-foundry"]').exists()).toBe(true)
      // Single-alternative line offers no meaningless dropdown.
      expect(wrapper.find('[data-testid="line-select-machine_shop"]').exists()).toBe(false)

      expect(wrapper.find('[data-testid="line-section-machine_shop"] [data-testid="line-always-on"]').exists()).toBe(
        true,
      )
      expect(wrapper.find('[data-testid="line-section-foundry"] [data-testid="line-always-on"]').exists()).toBe(false)

      // The multi-line explainer tells the player the lines are simultaneous.
      expect(wrapper.find('[data-testid="lines-explainer"]').exists()).toBe(true)
    })

    it('switching a line recipe sends only that line\'s recipe id and reloads', async () => {
      getBuildingDetail.mockResolvedValueOnce(twoLineDetail())
      setActiveRecipe.mockResolvedValueOnce(undefined)
      const switched = twoLineDetail()
      switched.lines[0].selected_recipe_id = 'foundry_smelt_conductive'
      switched.recipe = smeltConductive
      getBuildingDetail.mockResolvedValueOnce(switched)

      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerId: 'colony-1', buildingType: 'fabrication_complex' },
      })
      await flushPromises()

      const select = wrapper.find('[data-testid="line-select-foundry"]')
      ;(select.element as HTMLSelectElement).value = 'foundry_smelt_conductive'
      await select.trigger('change')
      await flushPromises()

      expect(setActiveRecipe).toHaveBeenCalledWith('colony-1', 'fabrication_complex', 'foundry_smelt_conductive')
      // The machine shop line is untouched by a foundry switch.
      const shop = wrapper.find('[data-testid="line-section-machine_shop"]')
      expect(shop.text()).toContain('components ×1')
      expect(wrapper.find('[data-testid="line-section-foundry"]').text()).toContain('conductive_metal ×2')
    })

    it('a single unnamed line renders as a plain "Recipe" section with no explainer', async () => {
      getBuildingDetail.mockResolvedValueOnce(
        makeDetail({
          lines: [
            {
              line: null,
              always_on: false,
              selected_recipe_id: 'research_recipe',
              alternatives: [
                {
                  recipe_id: 'research_recipe',
                  name: 'Basic Research',
                  inputs: [{ commodity_id: 'water', quantity: 1 }],
                  outputs: [{ commodity_id: 'research', quantity: 2 }],
                  cycle_sols: 1,
                },
              ],
            },
          ],
        }),
      )
      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerId: 'colony-1', buildingType: 'research_lab' },
      })
      await flushPromises()

      const section = wrapper.find('[data-testid="line-section-default"]')
      expect(section.exists()).toBe(true)
      expect(section.text()).toContain('Recipe: Basic Research')
      expect(section.text()).toContain('research ×2')
      expect(wrapper.find('[data-testid="lines-explainer"]').exists()).toBe(false)
    })
  })

  it('emits close when the backdrop is clicked', async () => {
    getBuildingDetail.mockResolvedValueOnce(makeDetail({}))
    const wrapper = mount(BuildingDetailsHud, {
      props: { ownerId: 'colony-1', buildingType: 'research_lab' },
    })
    await flushPromises()

    await wrapper.find('[data-testid="close-details-hud"]').trigger('click')
    expect(wrapper.emitted('close')).toBeTruthy()
  })

  describe('asPage presentation (floating window content, issue #339)', () => {
    it('renders as plain content with no backdrop when embedded in a floating window', async () => {
      getBuildingDetail.mockResolvedValueOnce(makeDetail({}))
      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerId: 'colony-1', buildingType: 'research_lab', asPage: true },
      })
      await flushPromises()

      expect(wrapper.find('[data-testid="facility-page"]').exists()).toBe(true)
      expect(wrapper.find('[data-testid="building-details-hud"]').exists()).toBe(false)
      expect(wrapper.find('[data-testid="close-details-hud"]').exists()).toBe(false)
      expect(wrapper.text()).toContain('Research Lab')
    })

    it('renders even without a building type selected yet, unlike modal mode', () => {
      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerId: 'colony-1', buildingType: null, asPage: true },
      })
      expect(wrapper.find('[data-testid="facility-page"]').exists()).toBe(true)
    })

    it('the Back button emits close, letting a floating-window host dismiss it', async () => {
      getBuildingDetail.mockResolvedValueOnce(makeDetail({}))
      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerId: 'colony-1', buildingType: 'research_lab', asPage: true },
      })
      await flushPromises()

      await wrapper.find('[data-testid="facility-back"]').trigger('click')
      expect(wrapper.emitted('close')).toBeTruthy()
    })
  })

  describe('ownerType="outpost" (navigation rework #7 phase 4)', () => {
    it('fetches outpost-scoped detail instead of colony detail', async () => {
      getOutpostBuildingDetail.mockResolvedValueOnce(makeDetail({ building_type: 'mining_outpost', name: 'Mining Outpost' }))
      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerType: 'outpost', ownerId: 'outpost-1', buildingType: 'mining_outpost' },
      })
      await flushPromises()

      expect(getOutpostBuildingDetail).toHaveBeenCalledWith('outpost-1', 'mining_outpost')
      expect(getBuildingDetail).not.toHaveBeenCalled()
      expect(wrapper.text()).toContain('Mining Outpost')
    })

    it('switches recipe via the outpost-scoped command', async () => {
      const recipeA = {
        recipe_id: 'mine_a',
        name: 'Mine A',
        inputs: [],
        outputs: [{ commodity_id: 'structural_ore', quantity: 10 }],
        cycle_sols: 1,
      }
      const recipeB = {
        recipe_id: 'mine_b',
        name: 'Mine B',
        inputs: [],
        outputs: [{ commodity_id: 'conductive_ore', quantity: 8 }],
        cycle_sols: 1,
      }
      getOutpostBuildingDetail.mockResolvedValueOnce(
        makeDetail({ building_type: 'mining_outpost', recipe: recipeA, available_recipes: [recipeA, recipeB] }),
      )
      const wrapper = mount(BuildingDetailsHud, {
        props: { ownerType: 'outpost', ownerId: 'outpost-1', buildingType: 'mining_outpost' },
      })
      await flushPromises()

      getOutpostBuildingDetail.mockResolvedValueOnce(
        makeDetail({ building_type: 'mining_outpost', recipe: recipeB, available_recipes: [recipeA, recipeB] }),
      )
      setOutpostActiveRecipe.mockResolvedValueOnce([])
      await wrapper.find('[data-testid="recipe-select"]').setValue('mine_b')
      await flushPromises()

      expect(setOutpostActiveRecipe).toHaveBeenCalledWith('outpost-1', 'mining_outpost', 'mine_b')
      expect(setActiveRecipe).not.toHaveBeenCalled()
    })
  })
})
