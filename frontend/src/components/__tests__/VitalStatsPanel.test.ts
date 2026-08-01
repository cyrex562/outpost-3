import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import VitalStatsPanel from '@/components/VitalStatsPanel.vue'

const baseProps = {
  population: 240,
  stability: 0.75,
  morale: 0.65,
  availableLabour: 12,
  populationTrend: [0, 0, 0],
  slotsUsed: 3,
  slotCapacity: 8,
  labourEmployed: 9,
  labourUnemployed: 3,
}

describe('VitalStatsPanel (UI-rework PR4; evolved from PopulationPanel)', () => {
  it('renders population, labour, build slots, and a stability label matching the band', () => {
    const wrapper = mount(VitalStatsPanel, { props: baseProps })
    expect(wrapper.find('[data-testid="population-count"]').text()).toBe('240')
    expect(wrapper.find('[data-testid="labour-available"]').text()).toBe('12')
    expect(wrapper.find('[data-testid="build-slots"]').text()).toBe('3 / 8')
    expect(wrapper.find('[data-testid="stability-label"]').text()).toContain('Stable')
    expect(wrapper.find('[data-testid="stability-label"]').classes()).toContain('stability-high')
  })

  it('renders morale as a stat distinct from stability (issue #382)', () => {
    const wrapper = mount(VitalStatsPanel, { props: baseProps })
    expect(wrapper.find('[data-testid="morale-label"]').text()).toContain('Content')
    expect(wrapper.find('[data-testid="morale-label"]').classes()).toContain('stability-high')
    expect(wrapper.find('[data-testid="morale-bar"]').attributes('aria-valuenow')).toBe('65')
  })

  it('labels low morale as Miserable', () => {
    const wrapper = mount(VitalStatsPanel, { props: { ...baseProps, morale: 0.1 } })
    expect(wrapper.find('[data-testid="morale-label"]').text()).toContain('Miserable')
    expect(wrapper.find('[data-testid="morale-label"]').classes()).toContain('stability-low')
  })

  it('labels low stability as Critical', () => {
    const wrapper = mount(VitalStatsPanel, {
      props: { ...baseProps, population: 100, stability: 0.1, availableLabour: 0, populationTrend: [] },
    })
    expect(wrapper.find('[data-testid="stability-label"]').text()).toContain('Critical')
    expect(wrapper.find('[data-testid="stability-label"]').classes()).toContain('stability-low')
  })

  it('splits the workforce into employed and unemployed, warning when anyone is idle (#305)', () => {
    const wrapper = mount(VitalStatsPanel, { props: baseProps })
    expect(wrapper.find('[data-testid="labour-breakdown"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="labour-employed"]').text()).toBe('9')
    expect(wrapper.find('[data-testid="labour-unemployed"]').text()).toBe('3')
    expect(wrapper.find('[data-testid="labour-unemployed"]').classes()).toContain('stat-warn')
  })

  it('does not warn when the whole workforce is employed (#305)', () => {
    const wrapper = mount(VitalStatsPanel, {
      props: { ...baseProps, labourEmployed: 12, labourUnemployed: 0 },
    })
    expect(wrapper.find('[data-testid="labour-unemployed"]').text()).toBe('0')
    expect(wrapper.find('[data-testid="labour-unemployed"]').classes()).not.toContain('stat-warn')
  })
})
