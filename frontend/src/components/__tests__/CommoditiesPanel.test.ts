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
          { commodity_id: 'iron', amount: 12.345, capacity: 100, net_per_turn: 3.2, reserved: 0 },
          { commodity_id: 'water', amount: 5, capacity: null, net_per_turn: -1.5, reserved: 0 },
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

  // ── Reserve control (issue #308) ──

  const row = (over: Partial<import('@/types/screen').StockpileRow> = {}) => ({
    commodity_id: 'biomass',
    amount: 500,
    capacity: 1000,
    net_per_turn: 0,
    reserved: 0,
    ...over,
  })

  it('shows a dash when nothing is reserved and the amount when something is', () => {
    const wrapper = mount(CommoditiesPanel, { props: { stockpile: [row()] } })
    expect(wrapper.find('[data-testid="reserve-edit-biomass"]').text()).toBe('—')

    const set = mount(CommoditiesPanel, { props: { stockpile: [row({ reserved: 250 })] } })
    const button = set.find('[data-testid="reserve-edit-biomass"]')
    expect(button.text()).toBe('250.0')
    expect(button.classes()).toContain('is-set')
  })

  it('emits the typed reserve on save', async () => {
    const wrapper = mount(CommoditiesPanel, { props: { stockpile: [row()] } })
    await wrapper.find('[data-testid="reserve-edit-biomass"]').trigger('click')
    await wrapper.find('[data-testid="reserve-input-biomass"]').setValue('250')
    await wrapper.find('[data-testid="reserve-save-biomass"]').trigger('click')

    expect(wrapper.emitted('set-reserve')).toEqual([['biomass', 250]])
    // Edit mode closes on save.
    expect(wrapper.find('[data-testid="reserve-input-biomass"]').exists()).toBe(false)
  })

  it('treats an emptied field as clearing the reserve', async () => {
    const wrapper = mount(CommoditiesPanel, { props: { stockpile: [row({ reserved: 250 })] } })
    await wrapper.find('[data-testid="reserve-edit-biomass"]').trigger('click')
    await wrapper.find('[data-testid="reserve-input-biomass"]').setValue('')
    await wrapper.find('[data-testid="reserve-save-biomass"]').trigger('click')

    expect(wrapper.emitted('set-reserve')).toEqual([['biomass', 0]])
  })

  it('refuses to emit a negative reserve and keeps the field open to fix', async () => {
    const wrapper = mount(CommoditiesPanel, { props: { stockpile: [row()] } })
    await wrapper.find('[data-testid="reserve-edit-biomass"]').trigger('click')
    await wrapper.find('[data-testid="reserve-input-biomass"]').setValue('-5')
    await wrapper.find('[data-testid="reserve-save-biomass"]').trigger('click')

    expect(wrapper.emitted('set-reserve')).toBeUndefined()
    expect(wrapper.find('[data-testid="reserve-input-biomass"]').exists()).toBe(true)
  })

  it('cancel discards the draft without emitting', async () => {
    const wrapper = mount(CommoditiesPanel, { props: { stockpile: [row({ reserved: 100 })] } })
    await wrapper.find('[data-testid="reserve-edit-biomass"]').trigger('click')
    await wrapper.find('[data-testid="reserve-input-biomass"]').setValue('999')
    await wrapper.find('[data-testid="reserve-cancel-biomass"]').trigger('click')

    expect(wrapper.emitted('set-reserve')).toBeUndefined()
    expect(wrapper.find('[data-testid="reserve-edit-biomass"]').text()).toBe('100.0')
  })
})
