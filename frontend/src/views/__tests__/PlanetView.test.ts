import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import PlanetView from '@/views/PlanetView.vue'
import type { PlanetHex, PlanetMap } from '@/services/tauriBridge'

const routerPush = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
}))

const getPlanetMap = vi.fn<[], Promise<PlanetMap>>()
vi.mock('@/services/tauriBridge', () => ({
  getPlanetMap: () => getPlanetMap(),
}))

function makeHex(overrides: Partial<PlanetHex>): PlanetHex {
  return {
    q: 0,
    r: 0,
    site_id: 'site-0',
    terrain: 'Plains',
    biome: 'Grassland',
    elevation: 0.5,
    temperature: 'Temperate',
    deposits: [],
    habitable: true,
    suitability: 10,
    occupied_by: null,
    occupant_colony_id: null,
    ...overrides,
  }
}

describe('PlanetView (map/nav plan phase A1: persistent planet map)', () => {
  beforeEach(() => {
    routerPush.mockReset()
    getPlanetMap.mockReset()
  })

  it('renders the planet hex map with colony nodes', async () => {
    getPlanetMap.mockResolvedValueOnce({
      seed: 1,
      radius: 1,
      hexes: [
        makeHex({ q: 0, r: 0, site_id: 's0' }),
        makeHex({
          q: 1,
          r: 0,
          site_id: 's1',
          occupied_by: 'Alpha Base',
          occupant_colony_id: 'colony-1',
        }),
      ],
    })
    const wrapper = mount(PlanetView)
    await flushPromises()

    expect(wrapper.find('[data-testid="planet-hex-map"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="colony-node-label-colony-1"]').text()).toBe('Alpha Base')
  })

  it('routes to the colony dashboard when a colony node is clicked', async () => {
    getPlanetMap.mockResolvedValueOnce({
      seed: 1,
      radius: 1,
      hexes: [
        makeHex({
          q: 0,
          r: 0,
          site_id: 's1',
          occupied_by: 'Alpha Base',
          occupant_colony_id: 'colony-1',
        }),
      ],
    })
    const wrapper = mount(PlanetView)
    await flushPromises()

    // The occupied hex group is the clickable colony node.
    await wrapper.get('.hex.occupied-clickable').trigger('click')

    expect(routerPush).toHaveBeenCalledWith({ name: 'colony', params: { colonyId: 'colony-1' } })
  })

  it('does not route when a non-colony (unoccupied) hex is clicked', async () => {
    getPlanetMap.mockResolvedValueOnce({
      seed: 1,
      radius: 1,
      hexes: [makeHex({ q: 0, r: 0, site_id: 's0', habitable: true })],
    })
    const wrapper = mount(PlanetView)
    await flushPromises()

    // Browse mode emits select for a habitable unoccupied hex too; the view's
    // occupant_colony_id guard must make that a no-op (no misroute).
    await wrapper.get('[data-testid="planet-hex-map"] g').trigger('click')

    expect(routerPush).not.toHaveBeenCalled()
  })

  it('surfaces an error when the planet map fails to load', async () => {
    getPlanetMap.mockRejectedValueOnce(new Error('no planet map — start a new game first'))
    const wrapper = mount(PlanetView)
    await flushPromises()

    expect(wrapper.find('[data-testid="planet-error"]').text()).toContain('no planet map')
  })
})
