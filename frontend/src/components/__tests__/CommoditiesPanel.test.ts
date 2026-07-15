import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import CommoditiesPanel from '@/components/CommoditiesPanel.vue'

describe('CommoditiesPanel (issue #169)', () => {
  it('shows a loading hint when the stockpile is null', () => {
    const wrapper = mount(CommoditiesPanel, { props: { stockpile: null } })
    expect(wrapper.find('[data-testid="commodity-table"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('Advance a turn to load commodity data.')
  })

  it('renders a row per commodity with net/sol formatting', () => {
    const wrapper = mount(CommoditiesPanel, {
      props: {
        stockpile: [
          { commodity_id: 'iron', amount: 12.345, capacity: 100, net_per_turn: 3.2 },
          { commodity_id: 'water', amount: 5, capacity: null, net_per_turn: -1.5 },
        ],
      },
    })
    const ironRow = wrapper.find('[data-testid="stock-row-iron"]')
    expect(ironRow.text()).toContain('12.3')
    expect(ironRow.text()).toContain('+3.20')

    const waterRow = wrapper.find('[data-testid="stock-row-water"]')
    expect(waterRow.text()).toContain('∞')
    expect(waterRow.text()).toContain('-1.50')
  })
})
