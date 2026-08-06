import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import SystemMapView from '@/views/SystemMapView.vue'
import type { SystemBody } from '@/services/tauriBridge'

const routerPush = vi.fn()
vi.mock('vue-router', () => ({
  useRoute: () => ({ query: {} }),
  useRouter: () => ({ push: routerPush }),
}))

const getSystemBodies = vi.fn<[], Promise<SystemBody[]>>()
const getSystemName = vi.fn<[], Promise<string>>()

vi.mock('@/services/tauriBridge', () => ({
  isTauri: false,
  getSystemBodies: () => getSystemBodies(),
  getSystemName: () => getSystemName(),
}))

function makeBody(overrides: Partial<SystemBody>): SystemBody {
  return {
    id: 'body-1',
    name: 'Body-1',
    kind: 'InnerPlanet',
    role: 'Unassigned',
    distance_au: 1.0,
    colonizable: false,
    atmosphere_density: 'Vacuum',
    atmosphere_hazard: 'None',
    temperature: 'Temperate',
    gravity_g: 1.0,
    radiation: 'Low',
    habitability: 40,
    habitability_modifier: 1.0,
    habitability_effective: 40,
    habitability_modifier_effective: 1.0,
    subtype: 'rocky',
    tidally_locked: false,
    axial_tilt_deg: 0,
    rotation_period_hours: 24,
    moon_count: 0,
    parent_body_name: null,
    category_modifiers: [],
    belt_profile: null,
    ...overrides,
  } as SystemBody
}

/** Read a body node's circle center from the rendered SVG. */
function circleCenter(wrapper: ReturnType<typeof mount>, id: string): { x: number; y: number } {
  const circle = wrapper.get(`[data-testid="body-node-${id}"] circle`)
  return { x: Number(circle.attributes('cx')), y: Number(circle.attributes('cy')) }
}

function dist(a: { x: number; y: number }, b: { x: number; y: number }): number {
  return Math.hypot(a.x - b.x, a.y - b.y)
}

describe('SystemMapView (system fixes B1: moons orbit their parent)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    getSystemBodies.mockReset()
    getSystemName.mockReset()
    getSystemName.mockResolvedValue('Vega')
    window.localStorage.clear()
  })

  it('nests a moon on a mini-orbit around its parent, not on its own star ring', async () => {
    const giant = makeBody({ id: 'giant-1', name: 'Giant-1', kind: 'GasGiant', distance_au: 5.0 })
    const moon = makeBody({
      id: 'giant-1-moon-1',
      name: 'Giant-1-Moon-1',
      kind: 'Moon',
      distance_au: 5.08,
      parent_body_name: 'Giant-1',
    })
    getSystemBodies.mockResolvedValueOnce([giant, moon])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    const giantPos = circleCenter(wrapper, 'giant-1')
    const moonPos = circleCenter(wrapper, 'giant-1-moon-1')

    // The moon sits on a mini-orbit hugging the giant (parent radius 14 +
    // base gap 10 + ring 0 = 24 world units away), far closer than its own
    // star-centered distance (5.08 AU * 100 = 508 units from the origin).
    expect(dist(moonPos, giantPos)).toBeCloseTo(24, 0)
    expect(dist(moonPos, { x: 0, y: 0 })).toBeGreaterThan(300)
  })

  it('hides a moon label until it is selected, but always labels the parent', async () => {
    const giant = makeBody({ id: 'giant-1', name: 'Giant-1', kind: 'GasGiant', distance_au: 5.0 })
    const moon = makeBody({
      id: 'giant-1-moon-1',
      name: 'Giant-1-Moon-1',
      kind: 'Moon',
      distance_au: 5.08,
      parent_body_name: 'Giant-1',
    })
    getSystemBodies.mockResolvedValueOnce([giant, moon])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    expect(wrapper.find('[data-testid="body-node-giant-1"] text').exists()).toBe(true)
    expect(wrapper.find('[data-testid="body-node-giant-1-moon-1"] text').exists()).toBe(false)

    // Selecting the moon reveals its label.
    await wrapper.get('[data-testid="body-node-giant-1-moon-1"]').trigger('click')
    expect(wrapper.find('[data-testid="body-node-giant-1-moon-1"] text').exists()).toBe(true)
  })

  it('renders a belt as a density-zoned annulus (paths, not a dot), opacity tracking density', async () => {
    const belt = makeBody({
      id: 'belt-1',
      name: 'Belt',
      kind: 'AsteroidBelt',
      distance_au: 2.5,
      belt_profile: {
        inner_au: 2.3,
        outer_au: 2.7,
        zones: [
          { start_deg: 0, sweep_deg: 180, density: 0.0 },
          { start_deg: 180, sweep_deg: 180, density: 1.0 },
        ],
      },
    })
    getSystemBodies.mockResolvedValueOnce([belt])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    const node = wrapper.get('[data-testid="body-node-belt-1"]')
    // Annulus sectors are <path>, and the belt has no single-dot <circle>.
    const paths = node.findAll('path')
    expect(paths).toHaveLength(2)
    expect(node.find('circle').exists()).toBe(false)

    // Zone opacity maps density 0 -> BELT_MIN_OPACITY (0.1), 1 -> BELT_MAX (0.8).
    const opacities = paths.map((p) => Number(p.attributes('fill-opacity'))).sort((a, b) => a - b)
    expect(opacities[0]).toBeCloseTo(0.1, 5)
    expect(opacities[1]).toBeCloseTo(0.8, 5)

    // Each zone is an annular-sector path: move, outer arc, inner arc, close —
    // two arcs winding oppositely (flag 1 then 0) so it fills the ring band,
    // not the whole disk.
    const d = paths[0].attributes('d') ?? ''
    expect(d.startsWith('M')).toBe(true)
    expect(d.trimEnd().endsWith('Z')).toBe(true)
    expect((d.match(/A /g) ?? []).length).toBe(2)
    expect(d).toMatch(/A [\d.-]+ [\d.-]+ 0 \d 1 /) // outer arc, sweep-flag 1
    expect(d).toMatch(/A [\d.-]+ [\d.-]+ 0 \d 0 /) // inner arc, sweep-flag 0
  })

  it('shows the belt span and zone count in the side panel when a belt is selected', async () => {
    const belt = makeBody({
      id: 'belt-1',
      name: 'Belt',
      kind: 'AsteroidBelt',
      distance_au: 2.5,
      belt_profile: {
        inner_au: 2.3,
        outer_au: 2.7,
        zones: [
          { start_deg: 0, sweep_deg: 180, density: 0.4 },
          { start_deg: 180, sweep_deg: 180, density: 0.6 },
        ],
      },
    })
    getSystemBodies.mockResolvedValueOnce([belt])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    await wrapper.get('[data-testid="body-node-belt-1"]').trigger('click')
    expect(wrapper.get('[data-testid="belt-span"]').text()).toContain('2.30–2.70 AU')
  })

  // Issue #313: the wire value is the engine's snake_case tag, which reads as
  // an implementation detail — the side panel should show the named archetype.
  it('shows the archetype label, not the raw snake_case tag', async () => {
    const ocean = makeBody({ id: 'ocean-1', name: 'Ocean World', subtype: 'ocean' })
    getSystemBodies.mockResolvedValueOnce([ocean])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    await wrapper.get('[data-testid="body-node-ocean-1"]').trigger('click')
    const archetype = wrapper.get('[data-testid="body-archetype"]')
    expect(archetype.text()).toBe('Ocean world')
    expect(archetype.text()).not.toContain('ocean_')
  })
})

describe('SystemMapView (#301: system + star name come from the engine)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    getSystemBodies.mockReset()
    getSystemName.mockReset()
    getSystemName.mockResolvedValue('Vega')
    window.localStorage.clear()
  })

  it('labels the heading and the star from the generated system name', async () => {
    getSystemBodies.mockResolvedValueOnce([makeBody({ id: 'planet-1', name: 'Vega-1' })])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    expect(wrapper.get('[data-testid="system-title"]').text()).toBe('Vega System')
    expect(wrapper.get('[data-testid="star-label"]').text()).toBe('VEGA')
    // The old hardcoded label is gone.
    expect(wrapper.text()).not.toContain('Kepler')
  })

  it('falls back to a generic label when the engine has no system name', async () => {
    getSystemName.mockResolvedValue('')
    getSystemBodies.mockResolvedValueOnce([makeBody({ id: 'planet-1' })])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    expect(wrapper.get('[data-testid="system-title"]').text()).toBe('Star System')
    expect(wrapper.get('[data-testid="star-label"]').text()).toBe('STAR')
  })

  it('still renders the map when the system-name lookup fails', async () => {
    getSystemName.mockRejectedValue(new Error('boom'))
    getSystemBodies.mockResolvedValueOnce([makeBody({ id: 'planet-1' })])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    expect(wrapper.get('[data-testid="system-title"]').text()).toBe('Star System')
    expect(wrapper.find('[data-testid="body-node-planet-1"]').exists()).toBe(true)
  })
})

describe('SystemMapView (map/nav plan: view body surface)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    getSystemBodies.mockReset()
    getSystemName.mockReset()
    getSystemName.mockResolvedValue('Vega')
    routerPush.mockReset()
    window.localStorage.clear()
  })

  it('offers "View Surface" for a planet and routes to the surface preview on click', async () => {
    const planet = makeBody({ id: 'planet-1', name: 'Chiron', kind: 'InnerPlanet', distance_au: 1.0 })
    getSystemBodies.mockResolvedValueOnce([planet])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    await wrapper.get('[data-testid="body-node-planet-1"]').trigger('click')
    const btn = wrapper.get('[data-testid="btn-view-surface"]')
    await btn.trigger('click')

    expect(routerPush).toHaveBeenCalledWith({
      name: 'surface',
      params: { bodyId: 'planet-1' },
      query: { name: 'Chiron' },
    })
  })

  it('offers "View Surface" for a moon', async () => {
    const moon = makeBody({ id: 'moon-1', name: 'Nessus', kind: 'Moon', parent_body_name: 'Chiron' })
    getSystemBodies.mockResolvedValueOnce([moon])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    await wrapper.get('[data-testid="body-node-moon-1"]').trigger('click')
    expect(wrapper.find('[data-testid="btn-view-surface"]').exists()).toBe(true)
  })

  it('hides "View Surface" for belts (no solid surface)', async () => {
    const belt = makeBody({
      id: 'belt-1',
      name: 'Belt',
      kind: 'AsteroidBelt',
      distance_au: 2.5,
      belt_profile: {
        inner_au: 2.3,
        outer_au: 2.7,
        zones: [{ start_deg: 0, sweep_deg: 360, density: 0.5 }],
      },
    })
    getSystemBodies.mockResolvedValueOnce([belt])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    await wrapper.get('[data-testid="body-node-belt-1"]').trigger('click')
    expect(wrapper.find('[data-testid="btn-view-surface"]').exists()).toBe(false)
  })
})

describe('SystemMapView (map fills available height)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerPush.mockReset()
    getSystemBodies.mockReset()
    getSystemName.mockReset()
    getSystemName.mockResolvedValue('Test System')
    window.localStorage.clear()
  })

  it('sizes the map panel from the layout, not a fixed pixel height', async () => {
    getSystemBodies.mockResolvedValueOnce([makeBody({ id: 'p1', name: 'P1' })])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    // The panel used to carry an inline `height: <n>px` driven by a
    // drag-resize grip. It now stretches to fill `.content`, so any inline
    // height would defeat the fill.
    const style = wrapper.get('.map-wrap').attributes('style')
    expect(style ?? '').not.toMatch(/height/)
  })

  it('no longer renders a resize grip', async () => {
    getSystemBodies.mockResolvedValueOnce([makeBody({ id: 'p1', name: 'P1' })])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    expect(wrapper.find('[data-testid="map-resize-grip"]').exists()).toBe(false)
  })

  it('restores pan/zoom from a layout entry written by the old grip build', async () => {
    // Pre-existing entries carry a `panelHeight` key that no longer maps to
    // anything. It must be ignored rather than rejecting the whole entry,
    // or an upgrading player silently loses their saved view.
    window.localStorage.setItem(
      'outpost3.system-map.layout',
      JSON.stringify({ x: -100, y: -200, w: 500, h: 600, panelHeight: 900 }),
    )
    getSystemBodies.mockResolvedValueOnce([makeBody({ id: 'p1', name: 'P1' })])
    const wrapper = mount(SystemMapView)
    await flushPromises()

    expect(wrapper.get('[data-testid="system-map-svg"]').attributes('viewBox')).toBe(
      '-100 -200 500 600',
    )
  })
})
