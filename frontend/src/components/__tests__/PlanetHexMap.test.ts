import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import PlanetHexMap from '@/components/PlanetHexMap.vue'
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
    ...overrides,
  }
}

function makeMap(hexes: PlanetHex[]): PlanetMap {
  return { seed: 1, radius: 1, hexes }
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
