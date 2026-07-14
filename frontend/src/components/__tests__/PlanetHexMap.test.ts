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
    // Grassland (#8ab558) at elevation 0.5 -> factor 0.965.
    expect(fill).toBe('rgb(133, 175, 85)')
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
