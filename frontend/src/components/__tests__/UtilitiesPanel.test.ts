import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import UtilitiesPanel from '@/components/UtilitiesPanel.vue'
import type { ResourceRow } from '@/types/screen'

const ROWS: ResourceRow[] = [
  { resource_id: 'housing', name: 'Housing Capacity', amount: 110, kind: 'capacity', unit: 'slots' },
  { resource_id: 'power', name: 'Power', amount: 24, kind: 'flow', unit: 'MW' },
  { resource_id: 'research', name: 'Research', amount: 1, kind: 'flow', unit: 'RP' },
]

describe('UtilitiesPanel (issue #304)', () => {
  it('renders one row per colony resource, with its name, amount, and unit', () => {
    const wrapper = mount(UtilitiesPanel, { props: { resources: ROWS } })
    expect(wrapper.find('[data-testid="utility-list"]').exists()).toBe(true)
    expect(wrapper.findAll('[data-testid^="utility-row-"]')).toHaveLength(3)
    expect(wrapper.get('[data-testid="utility-row-power"]').text()).toContain('Power')
    expect(wrapper.get('[data-testid="utility-amount-power"]').text()).toContain('MW')
    expect(wrapper.get('[data-testid="utility-amount-housing"]').text()).toContain('110')
  })

  it('distinguishes a standing capacity from per-sol throughput', () => {
    const wrapper = mount(UtilitiesPanel, { props: { resources: ROWS } })
    expect(wrapper.get('[data-testid="utility-row-housing"]').text()).toContain('capacity')
    expect(wrapper.get('[data-testid="utility-row-power"]').text()).toContain('per sol')
  })

  it('states that these are not tradeable, so they are not mistaken for cargo', () => {
    const wrapper = mount(UtilitiesPanel, { props: { resources: ROWS } })
    expect(wrapper.get('[data-testid="utilities-panel"]').text()).toContain('not tradeable')
  })

  it('shows a sub-unit trickle rather than rounding it away to zero', () => {
    // colony_hq's 1 RP/sol throttled by a shortfall must not read as "0".
    const wrapper = mount(UtilitiesPanel, {
      props: {
        resources: [
          { resource_id: 'research', name: 'Research', amount: 0.4, kind: 'flow', unit: 'RP' },
        ],
      },
    })
    expect(wrapper.get('[data-testid="utility-amount-research"]').text()).toContain('0.4')
  })

  it('distinguishes "not loaded yet" from "nothing produced this sol"', () => {
    const loading = mount(UtilitiesPanel, { props: { resources: null } })
    expect(loading.text()).toContain('Advance a turn')
    expect(loading.find('[data-testid="utilities-empty"]').exists()).toBe(false)

    const empty = mount(UtilitiesPanel, { props: { resources: [] } })
    expect(empty.find('[data-testid="utilities-empty"]').exists()).toBe(true)
  })
})
