import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import FoundColonyWizardView from '@/views/FoundColonyWizardView.vue'
import type { Command } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'

const getColonizeTargets = vi.fn()
const getSystemBodies = vi.fn()
const listBuildings = vi.fn()
const getPlanetMap = vi.fn()
const listSupplyPackages = vi.fn()

vi.mock('@/services/tauriBridge', () => ({
  getColonizeTargets: () => getColonizeTargets(),
  getSystemBodies: () => getSystemBodies(),
  listBuildings: () => listBuildings(),
  getPlanetMap: () => getPlanetMap(),
  listSupplyPackages: () => listSupplyPackages(),
}))

const routerPush = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => ({ query: {} }),
}))

const sendCommand = vi.fn<[Command], Promise<GameEvent[]>>()
const refreshColonyScreen = vi.fn()
// A single shared object (not a fresh literal per call) so tests can read
// back mutations the component makes, e.g. `gameStoreMock.toastMessage`
// after `finish()` sets it directly (mirrors the real Pinia store, which is
// also a single shared instance).
const gameStoreMock = {
  sendCommand: (cmd: Command) => sendCommand(cmd),
  busy: false,
  selectedColonyId: null as string | null,
  toastMessage: null as string | null,
  refreshColonyScreen: (id?: string | null) => refreshColonyScreen(id),
}
vi.mock('@/stores/game', () => ({
  useGameStore: () => gameStoreMock,
}))

const BODY = {
  body_id: 'mars',
  body_name: 'Mars',
  kind: 'Rocky',
  distance_au: 1.5,
  habitability: 80,
  can_found: true,
}

const HEX = {
  q: 0,
  r: 0,
  site_id: 'site-0',
  terrain: 'Plains',
  biome: 'Grassland',
  elevation: 0.5,
  temperature: 'Temperate',
  deposits: [],
  habitable: true,
  suitability: 90,
  occupied_by: null,
  occupant_colony_id: null,
}

const BUILDING_A = {
  id: 'water_well',
  name: 'Water Well',
  description: 'Pumps water.',
  category: 'extraction',
  slot_cost: 1,
  labor_per_turn: 1,
  construction_turns: 2,
  construction_cost: [],
  tech_prerequisite: null,
}

const BUILDING_B = {
  id: 'hydroponic_bay',
  name: 'Hydroponic Bay',
  description: 'Grows food.',
  category: 'agriculture',
  slot_cost: 2,
  labor_per_turn: 1,
  construction_turns: 3,
  construction_cost: [],
  tech_prerequisite: null,
}

const SUPPLY_PACKAGE = {
  id: 'standard',
  name: 'Standard',
  description: 'A balanced loadout.',
  commodities: [
    ['water', 50],
    ['food_ration', 20],
  ] as [string, number][],
}

/** Mount the wizard and drive it through step 1 (body) and step 2 (site), landing on step 3. */
async function mountAtStep3() {
  getColonizeTargets.mockResolvedValue([BODY])
  getSystemBodies.mockResolvedValue([])
  listBuildings.mockResolvedValue([BUILDING_A, BUILDING_B])
  getPlanetMap.mockResolvedValue({ seed: 1, radius: 1, hexes: [HEX] })
  listSupplyPackages.mockResolvedValue([SUPPLY_PACKAGE])

  const wrapper = mount(FoundColonyWizardView, {
    global: { stubs: { teleport: true } },
  })
  await flushPromises()

  await wrapper.find('[data-testid="body-card-mars"]').trigger('click')
  await wrapper.find('.btn.primary').trigger('click') // Next -> step 2
  await wrapper.find('g').trigger('click') // select the only hex
  await wrapper.find('.btn.primary').trigger('click') // Next -> step 3

  return wrapper
}

describe('FoundColonyWizardView step 3 loadout (#167)', () => {
  beforeEach(() => {
    getColonizeTargets.mockReset()
    getSystemBodies.mockReset()
    listBuildings.mockReset()
    getPlanetMap.mockReset()
    listSupplyPackages.mockReset()
    routerPush.mockReset()
    sendCommand.mockReset()
    refreshColonyScreen.mockReset()
    gameStoreMock.toastMessage = null
    gameStoreMock.selectedColonyId = null
  })

  it('pre-fills supply spinners from the default preset, scaled to starting population', async () => {
    const wrapper = await mountAtStep3()

    // startingPop defaults to 100, matching the preset's per-100 baseline exactly.
    const water = wrapper.find('[data-testid="supply-amount-water"]')
    const food = wrapper.find('[data-testid="supply-amount-food_ration"]')
    expect((water.element as HTMLInputElement).value).toBe('50')
    expect((food.element as HTMLInputElement).value).toBe('20')
  })

  it('rescales supply spinners when a different preset is picked after changing population', async () => {
    const wrapper = await mountAtStep3()

    await wrapper.find('[data-testid="population-input"]').setValue(200)
    await wrapper.find('[data-testid="supply-preset-standard"]').trigger('click')

    const water = wrapper.find('[data-testid="supply-amount-water"]')
    expect((water.element as HTMLInputElement).value).toBe('100')
  })

  it('pre-selects the landing kit so the default loadout matches what the engine would place', async () => {
    getColonizeTargets.mockResolvedValue([BODY])
    getSystemBodies.mockResolvedValue([])
    // hydroponic_bay is flagged as landing-kit content, water_well is not.
    listBuildings.mockResolvedValue([BUILDING_A, { ...BUILDING_B, starter_kit: true }])
    getPlanetMap.mockResolvedValue({ seed: 1, radius: 1, hexes: [HEX] })
    listSupplyPackages.mockResolvedValue([SUPPLY_PACKAGE])

    const wrapper = mount(FoundColonyWizardView, { global: { stubs: { teleport: true } } })
    await flushPromises()
    await wrapper.find('[data-testid="body-card-mars"]').trigger('click')
    await wrapper.find('.btn.primary').trigger('click')
    await wrapper.find('g').trigger('click')
    await wrapper.find('.btn.primary').trigger('click')

    // Only the flagged building is pre-selected, and its 2 slots are already
    // spent — so the step-3 gate is satisfied without the player touching it.
    const preview = wrapper.find('[data-testid="budget-preview"]')
    expect(preview.text()).toContain('2 / 10')
    expect(preview.classes()).not.toContain('over')
    const wellCount = wrapper.find('[data-testid="building-count-water_well"]')
    expect((wellCount.element as HTMLInputElement).value).toBe('0')
  })

  it('updates the budget preview as building counts change, and flags over-budget', async () => {
    const wrapper = await mountAtStep3()

    await wrapper.find('[data-testid="building-plus-water_well"]').trigger('click')
    let preview = wrapper.find('[data-testid="budget-preview"]')
    expect(preview.text()).toContain('1 / 10')
    expect(preview.classes()).not.toContain('over')

    // 1 water_well (1 slot) + 6 hydroponic_bay (2 slots each) = 13 > 10.
    for (let i = 0; i < 6; i += 1) {
      await wrapper.find('[data-testid="building-plus-hydroponic_bay"]').trigger('click')
    }

    preview = wrapper.find('[data-testid="budget-preview"]')
    expect(preview.text()).toContain('13 / 10')
    expect(preview.classes()).toContain('over')
  })

  it('sends supply_overrides and deploys a starter kit batch, omitting zeroed-out commodities', async () => {
    const wrapper = await mountAtStep3()

    // Zero out food_ration, and queue 2x water_well.
    await wrapper.find('[data-testid="supply-amount-food_ration"]').setValue(0)
    await wrapper.find('[data-testid="building-plus-water_well"]').trigger('click')
    await wrapper.find('[data-testid="building-plus-water_well"]').trigger('click')

    sendCommand.mockImplementation(async (cmd: Command) => {
      if (cmd.kind === 'found_colony_at_site') {
        return [{ kind: 'colony_founded', colony_id: 'colony-1' } as unknown as GameEvent]
      }
      if (cmd.kind === 'deploy_starter_kit') {
        return [
          { kind: 'building_constructed', colony_id: 'colony-1', building_type: 'water_well' } as unknown as GameEvent,
          { kind: 'building_constructed', colony_id: 'colony-1', building_type: 'water_well' } as unknown as GameEvent,
        ]
      }
      return []
    })

    await wrapper.find('.btn.primary').trigger('click') // Next -> step 4
    await wrapper.find('.btn.primary').trigger('click') // Found Colony
    await flushPromises()

    const foundCall = sendCommand.mock.calls.find(([cmd]) => cmd.kind === 'found_colony_at_site')
    expect(foundCall).toBeTruthy()
    const foundCmd = foundCall![0] as Extract<Command, { kind: 'found_colony_at_site' }>
    expect(foundCmd.starting_population).toBe(100)
    expect(foundCmd.supply_overrides).toEqual([['water', 50]])

    const kitCalls = sendCommand.mock.calls.filter(([cmd]) => cmd.kind === 'deploy_starter_kit')
    expect(kitCalls.length).toBe(1)
    const kitCmd = kitCalls[0][0] as Extract<Command, { kind: 'deploy_starter_kit' }>
    expect(kitCmd.colony_id).toBe('colony-1')
    expect(kitCmd.buildings).toEqual([
      ['water_well', 1],
      ['water_well', 1],
    ])

    expect(routerPush).toHaveBeenCalledWith('/colony')
  })

  it('still navigates when the starter kit deploy is rejected (rejection surfaced via toastMessage)', async () => {
    const wrapper = await mountAtStep3()

    await wrapper.find('[data-testid="building-plus-water_well"]').trigger('click')

    sendCommand.mockImplementation(async (cmd: Command) => {
      if (cmd.kind === 'found_colony_at_site') {
        return [{ kind: 'colony_founded', colony_id: 'colony-1' } as unknown as GameEvent]
      }
      // deploy_starter_kit rejected — engine returns no events.
      return []
    })

    await wrapper.find('.btn.primary').trigger('click') // Next -> step 4
    await wrapper.find('.btn.primary').trigger('click') // Found Colony
    await flushPromises()

    const kitCalls = sendCommand.mock.calls.filter(([cmd]) => cmd.kind === 'deploy_starter_kit')
    expect(kitCalls.length).toBe(1)
    expect(routerPush).toHaveBeenCalledWith('/colony')
  })
})
