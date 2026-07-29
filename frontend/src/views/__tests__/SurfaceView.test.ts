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
    width: 5,
    height: 4,
    edges: [],
    hexes: [makeHex({ q: 0, r: 0, site_id: 's0' }), makeHex({ q: 1, r: 0, site_id: 's1' })],
  }
}

/** A settled body's surface: one hex occupied by a colony. */
function makeSettledMap(): PlanetMap {
  return {
    seed: 42,
    width: 5,
    height: 4,
    edges: [],
    hexes: [
      makeHex({ q: 0, r: 0, site_id: 's0', occupied_by: 'Offworld', occupant_colony_id: 'col-7' }),
      makeHex({ q: 1, r: 0, site_id: 's1' }),
    ],
  }
}

describe('SurfaceView (map/nav plan: any body surface, live once settled)', () => {
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

  // Issue #300: the backend now returns the live stored surface for a settled
  // body, so this view has to distinguish "survey preview" from "real place".
  it('labels an unsettled body as a survey preview', async () => {
    getBodySurface.mockResolvedValueOnce(makeMap())
    const wrapper = mount(SurfaceView)
    await flushPromises()

    expect(wrapper.find('[data-testid="surface-preview-hint"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="surface-settled-hint"]').exists()).toBe(false)
  })

  it('labels a settled body as settled', async () => {
    getBodySurface.mockResolvedValueOnce(makeSettledMap())
    const wrapper = mount(SurfaceView)
    await flushPromises()

    expect(wrapper.find('[data-testid="surface-settled-hint"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="surface-preview-hint"]').exists()).toBe(false)
  })

  it('opens a colony dashboard when its node is clicked on a settled body', async () => {
    getBodySurface.mockResolvedValueOnce(makeSettledMap())
    const wrapper = mount(SurfaceView)
    await flushPromises()

    const map = wrapper.findComponent({ name: 'PlanetHexMap' })
    map.vm.$emit('select', makeHex({ occupant_colony_id: 'col-7', occupied_by: 'Offworld' }))
    await flushPromises()

    expect(routerPush).toHaveBeenCalledWith({ name: 'colony', params: { colonyId: 'col-7' } })
  })
})
