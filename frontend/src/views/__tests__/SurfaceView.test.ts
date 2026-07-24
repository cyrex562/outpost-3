import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import SurfaceView from '@/views/SurfaceView.vue'
import type { PlanetHex, PlanetMap } from '@/services/tauriBridge'

const routerPush = vi.fn()
const route = {
  params: { bodyId: 'body-1' } as Record<string, string>,
  query: { name: 'Chiron' } as Record<string, string>,
}
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => route,
}))

const getBodySurface = vi.fn<[string], Promise<PlanetMap>>()
vi.mock('@/services/tauriBridge', () => ({
  getBodySurface: (id: string) => getBodySurface(id),
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

function makeMap(): PlanetMap {
  return {
    seed: 42,
    radius: 8,
    edges: [],
    hexes: [makeHex({ q: 0, r: 0, site_id: 's0' }), makeHex({ q: 1, r: 0, site_id: 's1' })],
  }
}

describe('SurfaceView (map/nav plan: read-only body surface preview)', () => {
  beforeEach(() => {
    routerPush.mockReset()
    getBodySurface.mockReset()
    route.params = { bodyId: 'body-1' }
    route.query = { name: 'Chiron' }
  })

  it('fetches and renders the surface for the routed body', async () => {
    getBodySurface.mockResolvedValueOnce(makeMap())
    const wrapper = mount(SurfaceView)
    await flushPromises()

    expect(getBodySurface).toHaveBeenCalledWith('body-1')
    expect(wrapper.find('[data-testid="planet-hex-map"]').exists()).toBe(true)
    // Titles the page from the passed body name.
    expect(wrapper.find('.toolbar h2').text()).toContain('Chiron')
  })

  it('falls back to the body id when no name query param is present', async () => {
    route.query = {}
    getBodySurface.mockResolvedValueOnce(makeMap())
    const wrapper = mount(SurfaceView)
    await flushPromises()

    expect(wrapper.find('.toolbar h2').text()).toContain('body-1')
  })

  it('surfaces an error when the preview fails to load', async () => {
    getBodySurface.mockRejectedValueOnce(new Error('unknown body: body-1'))
    const wrapper = mount(SurfaceView)
    await flushPromises()

    expect(wrapper.find('[data-testid="surface-error"]').text()).toContain('unknown body')
  })

  it('navigates back to the system map', async () => {
    getBodySurface.mockResolvedValueOnce(makeMap())
    const wrapper = mount(SurfaceView)
    await flushPromises()

    await wrapper.get('[data-testid="btn-back-system"]').trigger('click')
    expect(routerPush).toHaveBeenCalledWith({ name: 'system' })
  })
})
