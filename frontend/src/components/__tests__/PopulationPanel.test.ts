import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import PopulationPanel from '@/components/PopulationPanel.vue'

describe('PopulationPanel (issue #169)', () => {
  it('renders population, labour, and a stability label matching the band', () => {
    const wrapper = mount(PopulationPanel, {
      props: { population: 240, stability: 0.75, availableLabour: 12, populationTrend: [0, 0, 0] },
    })
    expect(wrapper.find('[data-testid="population-count"]').text()).toBe('240')
    expect(wrapper.find('[data-testid="labour-available"]').text()).toBe('12')
    expect(wrapper.find('[data-testid="stability-label"]').text()).toContain('Stable')
    expect(wrapper.find('[data-testid="stability-label"]').classes()).toContain('stability-high')
  })

  it('labels low stability as Critical', () => {
    const wrapper = mount(PopulationPanel, {
      props: { population: 100, stability: 0.1, availableLabour: 0, populationTrend: [] },
    })
    expect(wrapper.find('[data-testid="stability-label"]').text()).toContain('Critical')
    expect(wrapper.find('[data-testid="stability-label"]').classes()).toContain('stability-low')
  })
})
