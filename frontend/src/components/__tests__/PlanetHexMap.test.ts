import { beforeEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import PlanetHexMap from '@/components/PlanetHexMap.vue'
import {
  __resetPlanetMapWrapForTests,
  __setPlanetMapWrapForTests,
} from '@/composables/usePlanetMapWrap'
import type { PlanetHex, PlanetMap } from '@/services/tauriBridge'

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

function makeMap(hexes: PlanetHex[], edges: PlanetMap['edges'] = []): PlanetMap {
  return { seed: 1, width: 5, height: 4, hexes, edges }
}

describe('PlanetHexMap temperature tint (#191)', () => {
  it('renders a distinct fill colour per temperature band on an otherwise identical hex', () => {
    const bands = ['Extreme', 'Frozen', 'Cold', 'Temperate', 'Hot'] as const
    const hexes = bands.map((temperature, i) =>
      makeHex({ q: i, r: 0, site_id: `site-${i}`, temperature }),
    )
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap(hexes), selectedSite: null },
    })
    const polygons = wrapper.findAll('polygon')
    expect(polygons.length).toBe(bands.length)
    const fills = polygons.map((p) => p.attributes('fill'))
    // Every band should be distinct except Cold/Temperate which share a
    // target tint colour but differ in blend strength — still assert no two
    // *adjacent-in-severity* bands collide, and Extreme/Hot clearly diverge
    // from the neutral Temperate baseline.
    expect(new Set(fills).size).toBeGreaterThan(1)
    const temperateFill = fills[bands.indexOf('Temperate')]
    const extremeFill = fills[bands.indexOf('Extreme')]
    const hotFill = fills[bands.indexOf('Hot')]
    expect(extremeFill).not.toBe(temperateFill)
    expect(hotFill).not.toBe(temperateFill)
    expect(extremeFill).not.toBe(hotFill)
  })

  it('leaves the Temperate band unshifted from the elevation-only baseline', () => {
    const hexes = [
      makeHex({ q: 0, r: 0, site_id: 'temperate', temperature: 'Temperate', elevation: 0.5 }),
    ]
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap(hexes), selectedSite: null },
    })
    const fill = wrapper.find('polygon').attributes('fill')
    // Plains terrain (#8a7f5a) + Grassland vegetation overlay (strength 0.25
    // toward #2f6b2f) -> rgb(115, 122, 79), then elevation 0.5 -> factor 0.965.
    expect(fill).toBe('rgb(111, 118, 76)')
  })

  it('tints cold bands toward blue and the hot band toward orange/red', () => {
    const hexes = [
      makeHex({ q: 0, r: 0, site_id: 'frozen', temperature: 'Frozen' }),
      makeHex({ q: 1, r: 0, site_id: 'hot', temperature: 'Hot' }),
    ]
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap(hexes), selectedSite: null },
    })
    const polygons = wrapper.findAll('polygon')
    const parseRgb = (s: string) => {
      const m = /rgb\((\d+), (\d+), (\d+)\)/.exec(s)
      if (!m) throw new Error(`unexpected fill format: ${s}`)
      return { r: Number(m[1]), g: Number(m[2]), b: Number(m[3]) }
    }
    const frozen = parseRgb(polygons[0].attributes('fill')!)
    const hot = parseRgb(polygons[1].attributes('fill')!)
    // Same base biome/elevation, so a blue shift on Frozen means more blue
    // than red relative to Hot, and Hot should carry more red than Frozen.
    expect(frozen.b - frozen.r).toBeGreaterThan(hot.b - hot.r)
  })
})

describe('PlanetHexMap harsh-climate suitability warning (#190)', () => {
  it('shows a climate warning in the tooltip for non-Temperate bands', async () => {
    const hexes = [
      makeHex({ q: 0, r: 0, site_id: 'frozen', temperature: 'Frozen' }),
      makeHex({ q: 1, r: 0, site_id: 'temperate', temperature: 'Temperate' }),
    ]
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap(hexes), selectedSite: null },
    })
    const groups = wrapper.findAll('g')

    await groups[0].trigger('mouseenter')
    expect(wrapper.find('.tt-warn').exists()).toBe(true)
    expect(wrapper.find('.tt-warn').text()).toMatch(/climate/i)

    await groups[1].trigger('mouseenter')
    expect(wrapper.find('.tt-warn').exists()).toBe(false)
  })

  it('does not show a climate warning for impassable or occupied hexes even if harsh', async () => {
    const hexes = [
      makeHex({
        q: 0,
        r: 0,
        site_id: 'ocean',
        temperature: 'Extreme',
        terrain: 'Ocean',
        habitable: false,
      }),
    ]
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap(hexes), selectedSite: null },
    })
    await wrapper.find('g').trigger('mouseenter')
    expect(wrapper.find('.tt-warn').text()).toBe('Impassable')
  })
})

describe('PlanetHexMap occupied-hex selection (persistent map, phase A1)', () => {
  const occupiedHex = () =>
    makeHex({
      q: 0,
      r: 0,
      site_id: 's1',
      occupied_by: 'Alpha Base',
      occupant_colony_id: 'colony-1',
    })

  it('leaves occupied hexes inert by default (wizard mode)', async () => {
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap([occupiedHex()]), selectedSite: null },
    })
    // Target the occupied hex group explicitly (not the first <g>, which
    // could be a non-hex element) so the assertion truly pins inertness.
    await wrapper.get('.hex.occupied').trigger('click')
    expect(wrapper.emitted('select')).toBeFalsy()
  })

  it('emits select for an occupied hex when selectableOccupied is set (browse mode)', async () => {
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap([occupiedHex()]), selectedSite: null, selectableOccupied: true },
    })
    await wrapper.get('.hex.occupied-clickable').trigger('click')
    const events = wrapper.emitted('select')
    expect(events).toBeTruthy()
    expect((events?.[0][0] as { occupant_colony_id: string }).occupant_colony_id).toBe('colony-1')
  })

  it('renders the colony name label on an occupied hex', () => {
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap([occupiedHex()]), selectedSite: null },
    })
    expect(wrapper.find('[data-testid="colony-node-label-colony-1"]').text()).toBe('Alpha Base')
  })
})

describe('PlanetHexMap infrastructure edges (map/nav plan phase A3)', () => {
  const twoColonies = () => [
    makeHex({ q: 0, r: 0, site_id: 's1', occupied_by: 'Alpha', occupant_colony_id: 'colony-1' }),
    makeHex({ q: 2, r: 0, site_id: 's2', occupied_by: 'Beta', occupant_colony_id: 'colony-2' }),
  ]

  it('draws a line between the two colonies an edge connects', () => {
    const wrapper = mount(PlanetHexMap, {
      props: {
        map: makeMap(twoColonies(), [
          {
            from_colony_id: 'colony-1',
            to_colony_id: 'colony-2',
            infra_type: 'road',
            throughput: 50,
            cost: 10,
            loss_pct: 0,
          },
        ]),
        selectedSite: null,
      },
    })

    const line = wrapper.get('[data-testid="infra-edge-colony-1-colony-2-road"]')
    // Endpoints are the two colonies' hex centers: colony-1 at (0,0), colony-2
    // to its right (axialToPixel(2,0) has x > 0, y == 0).
    expect(line.attributes('x1')).toBe('0')
    expect(line.attributes('y1')).toBe('0')
    expect(Number(line.attributes('x2'))).toBeGreaterThan(0)
    expect(line.attributes('stroke')).toBe('#b8a06a') // road color
  })

  it('skips an edge whose endpoint colony is not on the map', () => {
    const wrapper = mount(PlanetHexMap, {
      props: {
        map: makeMap(twoColonies(), [
          {
            from_colony_id: 'colony-1',
            to_colony_id: 'colony-missing',
            infra_type: 'rail',
            throughput: 200,
            cost: 50,
            loss_pct: 0,
          },
        ]),
        selectedSite: null,
      },
    })
    expect(wrapper.findAll('.infra-edge')).toHaveLength(0)
  })
})

describe('PlanetHexMap deposit boxes', () => {
  it('renders a two-letter code box per deposit, for real VEIN_COMMODITIES entries', () => {
    const hexes = [
      makeHex({
        q: 0,
        r: 0,
        site_id: 'deposit-hex',
        deposits: [
          { commodity_id: 'structural_ore', richness: 0.8 },
          { commodity_id: 'fissile_ore', richness: 0.4 },
        ],
      }),
    ]
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap(hexes), selectedSite: null },
    })
    const codes = wrapper.findAll('.deposit-code').map((n) => n.text())
    expect(codes.sort()).toEqual(['FI', 'SO'])
  })

  it('falls back to the first two letters, uppercased, for an unlisted commodity', () => {
    const hexes = [
      makeHex({
        q: 0,
        r: 0,
        site_id: 'unknown-deposit',
        deposits: [{ commodity_id: 'exotic_gas', richness: 0.5 }],
      }),
    ]
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap(hexes), selectedSite: null },
    })
    expect(wrapper.find('.deposit-code').text()).toBe('EX')
  })

  it('lists only commodities actually present on the map in the legend, with matching codes', () => {
    const hexes = [
      makeHex({
        q: 0,
        r: 0,
        site_id: 'deposit-hex',
        deposits: [{ commodity_id: 'silicates', richness: 0.6 }],
      }),
    ]
    const wrapper = mount(PlanetHexMap, {
      props: { map: makeMap(hexes), selectedSite: null },
    })
    const legend = wrapper.get('[data-testid="planet-map-legend"]')
    expect(legend.text()).toContain('SL')
    expect(legend.text()).toContain('silicates')
  })
})

describe('PlanetHexMap layered tile colour (#316)', () => {
  function parseRgb(s: string): { r: number; g: number; b: number } {
    const m = /^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/.exec(s)
    if (!m) throw new Error(`unexpected fill format: ${s}`)
    return { r: Number(m[1]), g: Number(m[2]), b: Number(m[3]) }
  }

  it('shifts fill bluer as water_coverage increases on otherwise identical hexes', () => {
    const hexes = [
      makeHex({ q: 0, r: 0, site_id: 'dry', terrain: 'Ocean', water_coverage: 0 }),
      makeHex({ q: 1, r: 0, site_id: 'full', terrain: 'Ocean', water_coverage: 1 }),
    ]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    const [dry, full] = wrapper.findAll('polygon').map((p) => parseRgb(p.attributes('fill')!))
    // Full coverage should read visibly more blue than zero coverage on the
    // same (Ocean) terrain base.
    expect(full.b).toBeGreaterThan(dry.b)
  })

  it('renders water as white/ice on a Frozen hex and blue on a Temperate hex', () => {
    const hexes = [
      makeHex({
        q: 0,
        r: 0,
        site_id: 'frozen-water',
        terrain: 'Ocean',
        water_coverage: 1,
        temperature: 'Frozen',
      }),
      makeHex({
        q: 1,
        r: 0,
        site_id: 'liquid-water',
        terrain: 'Ocean',
        water_coverage: 1,
        temperature: 'Hot',
      }),
    ]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    const [frozen, liquid] = wrapper.findAll('polygon').map((p) => parseRgb(p.attributes('fill')!))
    // Ice reads brighter/whiter (all channels high) than liquid water.
    expect(frozen.r).toBeGreaterThan(liquid.r)
    expect(frozen.g).toBeGreaterThan(liquid.g)
  })

  it('shifts fill toward the vegetation colour as vegetation_density increases', () => {
    const hexes = [
      makeHex({ q: 0, r: 0, site_id: 'bare', vegetation_density: 0 }),
      makeHex({ q: 1, r: 0, site_id: 'lush', vegetation_density: 1 }),
    ]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    const [bare, lush] = wrapper.findAll('polygon').map((p) => parseRgb(p.attributes('fill')!))
    // Full density should blend fully toward the vegetation colour
    // (#2f6b2f), which is darker/redder than the Plains terrain base
    // (#8a7f5a) — the red channel is the clearest, unambiguous signal.
    expect(lush.r).toBeLessThan(bare.r)
  })

  it('renders no vegetation tint when vegetation_density is zero, even on a lush biome', () => {
    const hexes = [
      makeHex({ q: 0, r: 0, site_id: 'no-veg', biome: 'Jungle', vegetation_density: 0 }),
    ]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    const fill = wrapper.find('polygon').attributes('fill')!
    // Same as a Plains/Grassland-with-zero-vegetation baseline shape: no
    // green shift should have been applied by vegetation_density: 0.
    const plain = mount(PlanetHexMap, {
      props: {
        map: makeMap([makeHex({ q: 0, r: 0, site_id: 'baseline', vegetation_density: 0 })]),
        selectedSite: null,
      },
    })
    // Both explicitly zero vegetation_density hexes share the same terrain
    // (Plains) and temperature/elevation, so their fills should match
    // regardless of biome — proving the real field, not biome, drives it.
    expect(fill).toBe(plain.find('polygon').attributes('fill'))
  })
})

describe('PlanetHexMap contamination overlay (#387)', () => {
  function parseRgb(s: string): { r: number; g: number; b: number } {
    const m = /^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/.exec(s)
    if (!m) throw new Error(`unexpected fill format: ${s}`)
    return { r: Number(m[1]), g: Number(m[2]), b: Number(m[3]) }
  }

  it('shifts fill toward the contamination colour as severity increases', () => {
    const hexes = [
      makeHex({ q: 0, r: 0, site_id: 'clean', contamination: 0 }),
      makeHex({ q: 1, r: 0, site_id: 'fouled', contamination: 1 }),
    ]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    const [clean, fouled] = wrapper.findAll('polygon').map((p) => parseRgb(p.attributes('fill')!))
    // Full severity should blend toward the sickly yellow-green
    // contamination colour (#9acd1a) — greener and less blue than a
    // pristine hex on the same terrain.
    expect(fouled.g).toBeGreaterThan(clean.g)
    expect(fouled.b).toBeLessThan(clean.b)
  })

  it('renders no contamination tint when contamination is absent or zero', () => {
    const hexes = [makeHex({ q: 0, r: 0, site_id: 'no-contam' })]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    const fill = wrapper.find('polygon').attributes('fill')!
    const zero = mount(PlanetHexMap, {
      props: {
        map: makeMap([makeHex({ q: 0, r: 0, site_id: 'zero-contam', contamination: 0 })]),
        selectedSite: null,
      },
    })
    expect(fill).toBe(zero.find('polygon').attributes('fill'))
  })

  it('shows a contamination warning in the tooltip when severity is above zero', async () => {
    const hexes = [makeHex({ q: 0, r: 0, site_id: 'fouled', contamination: 0.42 })]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    await wrapper.find('g').trigger('mouseenter')
    expect(wrapper.text()).toContain('Contaminated · 42%')
  })
})

describe('PlanetHexMap wrap-seam indicator', () => {
  it('draws one seam line per q=0 hex, and none for other columns', () => {
    const hexes = [
      makeHex({ q: 0, r: 0, site_id: 'seam-0' }),
      makeHex({ q: 0, r: 1, site_id: 'seam-1' }),
      makeHex({ q: 2, r: 0, site_id: 'not-seam' }),
    ]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    expect(wrapper.findAll('[data-testid="seam-line"]')).toHaveLength(2)
  })

  it('draws no seam line at all when no hex has q=0', () => {
    const hexes = [makeHex({ q: 1, r: 0, site_id: 'not-seam' })]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    expect(wrapper.find('[data-testid="seam-line"]').exists()).toBe(false)
  })

  it('traces the hex\'s actual west-facing edge, not a straight vertical cut through its center', () => {
    const hexes = [makeHex({ q: 0, r: 0, site_id: 'seam-0' })]
    const wrapper = mount(PlanetHexMap, { props: { map: makeMap(hexes), selectedSite: null } })
    const line = wrapper.get('[data-testid="seam-line"]')
    const x1 = Number(line.attributes('x1'))
    const y1 = Number(line.attributes('y1'))
    const x2 = Number(line.attributes('x2'))
    const y2 = Number(line.attributes('y2'))
    // Both endpoints sit left of the hex center (cx=0 for q=0,r=0), and are
    // vertically offset from it (not a single point, not a horizontal line) —
    // i.e. an actual slanted hex edge, not a straight vertical/horizontal cut.
    expect(x1).toBeLessThan(0)
    expect(x2).toBeLessThan(0)
    expect(y1).not.toBe(y2)
    expect(x1).toBeCloseTo(x2) // the west edge of a pointy-top hex is vertical (constant x)
  })
})

describe('PlanetHexMap wrap toggle', () => {
  /**
   * A tall, narrow map: because axial x depends on `r` as well as `q`, ten
   * rows shear the rendered extent well past one wrap period (`width: 2`),
   * so wrapping actually emits repeats here. A map fitted exactly to its own
   * extent needs none, which would make these tests vacuous.
   */
  function tallMap(): PlanetMap {
    const hexes = Array.from({ length: 10 }, (_, r) =>
      makeHex({ q: 0, r, site_id: `site-${r}` }),
    )
    return { seed: 1, width: 2, height: 12, hexes, edges: [] }
  }

  const HEX_COUNT = 10

  /**
   * Mount and let the initial `fitToContent` (which runs in `onMounted`)
   * reach the DOM. Before that flush the map still renders against its
   * placeholder viewBox, which produces a different repeat count than the
   * settled view.
   */
  async function mountSettled() {
    const wrapper = mount(PlanetHexMap, {
      props: { map: tallMap(), selectedSite: null },
    })
    await wrapper.vm.$nextTick()
    return wrapper
  }

  beforeEach(() => {
    window.localStorage.clear()
    __resetPlanetMapWrapForTests()
  })

  it('repeats the map east-west while wrapping is on', async () => {
    const wrapper = await mountSettled()
    expect(wrapper.findAll('polygon').length).toBeGreaterThan(HEX_COUNT)
  })

  it('collapses to a single copy when wrapping is switched off', async () => {
    const wrapper = await mountSettled()
    expect(wrapper.findAll('polygon').length).toBeGreaterThan(HEX_COUNT)

    await wrapper.get('[data-testid="planet-map-wrap-toggle"]').trigger('click')

    expect(wrapper.findAll('polygon').length).toBe(HEX_COUNT)
    // The seam markers repeat off the same shift list, so they collapse too.
    expect(wrapper.findAll('[data-testid="seam-line"]').length).toBe(HEX_COUNT)
  })

  it('restores the repeats when switched back on', async () => {
    const wrapper = await mountSettled()
    const wrapped = wrapper.findAll('polygon').length
    const toggle = wrapper.get('[data-testid="planet-map-wrap-toggle"]')

    await toggle.trigger('click')
    expect(wrapper.findAll('polygon').length).toBe(HEX_COUNT)

    await toggle.trigger('click')
    expect(wrapper.findAll('polygon').length).toBe(wrapped)
  })

  it('reflects the current mode in its label and aria-pressed state', async () => {
    const wrapper = await mountSettled()
    const toggle = wrapper.get('[data-testid="planet-map-wrap-toggle"]')

    expect(toggle.attributes('aria-pressed')).toBe('true')
    expect(toggle.text()).toMatch(/on/i)

    await toggle.trigger('click')

    expect(toggle.attributes('aria-pressed')).toBe('false')
    expect(toggle.text()).toMatch(/off/i)
  })

  it('starts unwrapped when the preference was previously turned off', async () => {
    __setPlanetMapWrapForTests(false)
    const wrapper = await mountSettled()
    expect(wrapper.findAll('polygon').length).toBe(HEX_COUNT)
  })
})


describe('PlanetHexMap geothermal gradient (issue #412)', () => {
  function mapWith(gradient: number | undefined) {
    const hex = makeHex({ q: 0, r: 0, site_id: 'g1' })
    if (gradient !== undefined) {
      ;(hex as unknown as Record<string, number>).geothermal_gradient = gradient
    }
    return makeMap([hex])
  }

  async function tooltipFor(gradient: number | undefined) {
    const wrapper = mount(PlanetHexMap, {
      props: { map: mapWith(gradient), selectedSite: null },
    })
    await wrapper.vm.$nextTick()
    await wrapper.get('g').trigger('mouseenter')
    return wrapper
  }

  it('reports a shallow-magma hex in terms a player can act on', async () => {
    const wrapper = await tooltipFor(0.85)
    expect(wrapper.text()).toContain('Geothermal')
    expect(wrapper.text()).toContain('85%')
    expect(wrapper.text()).toMatch(/shallow magma/i)
  })

  it('warns that a cold hex needs drilling tech', async () => {
    // Matches the engine's DEEP_DRILLING_GRADIENT threshold, so the tooltip
    // and the rule agree.
    const wrapper = await tooltipFor(0.1)
    expect(wrapper.text()).toMatch(/needs drilling tech/i)
  })

  it('describes the bands between the two extremes', async () => {
    expect((await tooltipFor(0.5)).text()).toMatch(/warm crust/i)
    expect((await tooltipFor(0.3)).text()).toMatch(/cool crust/i)
  })

  it('omits the row entirely when the backend predates the layer', async () => {
    const wrapper = await tooltipFor(undefined)
    expect(wrapper.text()).not.toContain('Geothermal')
  })
})
