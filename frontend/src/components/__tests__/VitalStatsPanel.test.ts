import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import VitalStatsPanel from '@/components/VitalStatsPanel.vue'

const baseProps = {
  population: 240,
  stability: 0.75,
  availableLabour: 12,
  populationTrend: [0, 0, 0],
  slotsUsed: 3,
  slotCapacity: 8,
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

  it('labels low stability as Critical', () => {
    const wrapper = mount(VitalStatsPanel, {
      props: { ...baseProps, population: 100, stability: 0.1, availableLabour: 0, populationTrend: [] },
    })
    expect(wrapper.find('[data-testid="stability-label"]').text()).toContain('Critical')
    expect(wrapper.find('[data-testid="stability-label"]').classes()).toContain('stability-low')
  })
})
