import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import SystemBodiesView from '@/views/SystemBodiesView.vue'
import type { SystemBody } from '@/services/tauriBridge'

const routerPush = vi.fn()

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
}))

const getSystemBodies = vi.fn<[], Promise<SystemBody[]>>()

vi.mock('@/services/tauriBridge', () => ({
  getSystemBodies: () => getSystemBodies(),
}))

function makeBody(overrides: Partial<SystemBody>): SystemBody {
  return {
    id: 'body-1',
    name: 'Luna',
    kind: 'Moon',
    role: 'Satellite',
    distance_au: 1.0,
    colonizable: true,
    atmosphere_density: 0,
    atmosphere_hazard: null,
    temperature: 200,
    gravity_g: 0.16,
    radiation: 0.1,
    habitability: 0.4,
    habitability_modifier: 1.0,
    habitability_effective: 0.4,
    habitability_modifier_effective: 1.0,
    subtype: null,
    tidally_locked: true,
    axial_tilt_deg: 0,
    rotation_period_hours: 655,
    moon_count: 0,
    parent_body_name: 'Earth',
    category_modifiers: [],
    ...overrides,
  } as SystemBody
}

describe('SystemBodiesView (navigation rework #7 phase 3: system bodies list)', () => {
  beforeEach(() => {
    routerPush.mockReset()
    getSystemBodies.mockReset()
  })

  it('lists bodies sorted by distance from the star', async () => {
    getSystemBodies.mockResolvedValueOnce([
      makeBody({ id: 'far', name: 'Outer World', distance_au: 5.0 }),
      makeBody({ id: 'near', name: 'Inner World', distance_au: 0.5 }),
    ])
    const wrapper = mount(SystemBodiesView)
    await flushPromises()

    const rows = wrapper.findAll('[data-testid^="body-row-"]')
    expect(rows.map((r) => r.attributes('data-testid'))).toEqual(['body-row-near', 'body-row-far'])
  })

  it('navigates to the system map with the body preselected when a row is clicked', async () => {
    getSystemBodies.mockResolvedValueOnce([makeBody({ id: 'body-1', name: 'Luna' })])
    const wrapper = mount(SystemBodiesView)
    await flushPromises()

    await wrapper.get('[data-testid="body-row-body-1"]').trigger('click')

    expect(routerPush).toHaveBeenCalledWith({ path: '/system', query: { body: 'body-1' } })
  })
})
