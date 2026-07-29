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

const sendCommand = vi.fn()
const gameStoreMock = {
  sendCommand: (cmd: unknown) => sendCommand(cmd),
  toastMessage: null as string | null,
}
vi.mock('@/stores/game', () => ({
  useGameStore: () => gameStoreMock,
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
    sendCommand.mockReset()
    gameStoreMock.toastMessage = null
  })

  function twoColonyMap(edges: PlanetMap['edges'] = []): PlanetMap {
    return {
      seed: 1,
      width: 5,
      height: 4,
      edges,
      hexes: [
        makeHex({ q: 0, r: 0, site_id: 's1', occupied_by: 'Alpha', occupant_colony_id: 'colony-1' }),
        makeHex({ q: 2, r: 0, site_id: 's2', occupied_by: 'Beta', occupant_colony_id: 'colony-2' }),
      ],
    }
  }

  it('renders the planet hex map with colony nodes', async () => {
    getPlanetMap.mockResolvedValueOnce({
      seed: 1,
      width: 5,
      height: 4,
      edges: [],
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
      width: 5,
      height: 4,
      edges: [],
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
      width: 5,
      height: 4,
      edges: [],
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

  it('builds infrastructure between two picked colonies (map/nav plan phase A3b)', async () => {
    getPlanetMap.mockResolvedValue(twoColonyMap())
    sendCommand.mockResolvedValueOnce([{ kind: 'infrastructure_built' }])
    const wrapper = mount(PlanetView)
    await flushPromises()

    await wrapper.get('[data-testid="btn-toggle-build"]').trigger('click')
    // Pick the two colony nodes as the from/to endpoints.
    const nodes = wrapper.findAll('.hex.occupied-clickable')
    await nodes[0].trigger('click')
    await nodes[1].trigger('click')
    await wrapper.get('[data-testid="infra-type-select"]').setValue('rail')
    await wrapper.get('[data-testid="btn-build"]').trigger('click')
    await flushPromises()

    expect(sendCommand).toHaveBeenCalledWith({
      kind: 'build_infrastructure',
      from_colony: 'colony-1',
      to_colony: 'colony-2',
      infra_type: 'rail',
    })
    // Refreshes the map after building (initial load + post-build refresh).
    expect(getPlanetMap).toHaveBeenCalledTimes(2)
  })

  it('keeps the Build button disabled until two distinct colonies are picked', async () => {
    getPlanetMap.mockResolvedValue(twoColonyMap())
    const wrapper = mount(PlanetView)
    await flushPromises()

    await wrapper.get('[data-testid="btn-toggle-build"]').trigger('click')
    expect(wrapper.get('[data-testid="btn-build"]').attributes('disabled')).toBeDefined()

    await wrapper.findAll('.hex.occupied-clickable')[0].trigger('click')
    // Only one endpoint picked — still disabled.
    expect(wrapper.get('[data-testid="btn-build"]').attributes('disabled')).toBeDefined()
  })

  it('demolishes an existing edge from the edge list', async () => {
    getPlanetMap.mockResolvedValue(
      twoColonyMap([
        { from_colony_id: 'colony-1', to_colony_id: 'colony-2', infra_type: 'road', throughput: 50, cost: 10 },
      ]),
    )
    sendCommand.mockResolvedValueOnce([{ kind: 'infrastructure_demolished' }])
    const wrapper = mount(PlanetView)
    await flushPromises()

    await wrapper.get('[data-testid="btn-toggle-build"]').trigger('click')
    await wrapper.get('[data-testid="edge-list"] .btn.danger').trigger('click')
    await flushPromises()

    expect(sendCommand).toHaveBeenCalledWith({
      kind: 'demolish_infrastructure',
      from_colony: 'colony-1',
      to_colony: 'colony-2',
    })
    // Refreshes the map after demolishing (initial load + post-demolish).
    expect(getPlanetMap).toHaveBeenCalledTimes(2)
  })

  it('surfaces the rejection reason when a build is refused (empty events)', async () => {
    getPlanetMap.mockResolvedValue(twoColonyMap())
    sendCommand.mockResolvedValueOnce([]) // engine rejected the command
    gameStoreMock.toastMessage = 'Error: colonies are not both on the planet map.'
    const wrapper = mount(PlanetView)
    await flushPromises()

    await wrapper.get('[data-testid="btn-toggle-build"]').trigger('click')
    const nodes = wrapper.findAll('.hex.occupied-clickable')
    await nodes[0].trigger('click')
    await nodes[1].trigger('click')
    await wrapper.get('[data-testid="btn-build"]').trigger('click')
    await flushPromises()

    expect(wrapper.get('[data-testid="planet-error"]').text()).toContain('not both on the planet map')
    // A rejected build did not refresh (only the initial load ran).
    expect(getPlanetMap).toHaveBeenCalledTimes(1)
  })

  it('does not route while in build mode (colony clicks pick endpoints instead)', async () => {
    getPlanetMap.mockResolvedValue(twoColonyMap())
    const wrapper = mount(PlanetView)
    await flushPromises()

    await wrapper.get('[data-testid="btn-toggle-build"]').trigger('click')
    await wrapper.findAll('.hex.occupied-clickable')[0].trigger('click')

    expect(routerPush).not.toHaveBeenCalled()
  })
})
