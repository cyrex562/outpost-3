import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import BuildingsPanel from '@/components/BuildingsPanel.vue'
import type { BuildingRow } from '@/types/screen'

function makeRow(overrides: Partial<BuildingRow>): BuildingRow {
  return {
    building_type: 'smelter',
    labour_assigned: 5,
    slot_cost: 2,
    full_capacity: true,
    ...overrides,
  }
}

describe('BuildingsPanel status derivation (#169)', () => {
  it('shows Idle when no labour is assigned', () => {
    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [makeRow({ labour_assigned: 0, full_capacity: false })],
        slotsUsed: 0,
        slotCapacity: 10,
        labourAvailable: 10,
        labourTotal: 10,
      },
    })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Idle')
  })

  it('shows Running when labour is assigned and at full capacity', () => {
    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [makeRow({ labour_assigned: 5, full_capacity: true })],
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 5,
        labourTotal: 10,
      },
    })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Running')
  })

  it('shows Partial when labour is assigned but not at full capacity', () => {
    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [makeRow({ labour_assigned: 2, full_capacity: false })],
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 8,
        labourTotal: 10,
      },
    })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Partial')
  })

  it('shows a loading hint when buildings is null', () => {
    const wrapper = mount(BuildingsPanel, {
      props: { buildings: null, slotsUsed: 0, slotCapacity: 0, labourAvailable: 0, labourTotal: 0 },
    })
    expect(wrapper.find('[data-testid="building-list"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('No building data loaded.')
  })
})

describe('BuildingsPanel labour assignment', () => {
  it('emits assign-labour with the drafted value and clears the draft', async () => {
    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [makeRow({ building_type: 'hydroponic_bay', labour_assigned: 3 })],
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 7,
        labourTotal: 10,
      },
    })
    const input = wrapper.find('[data-testid="labour-input-hydroponic_bay"]')
    await input.setValue('8')
    const assignBtn = wrapper.find('[data-testid="assign-labour-hydroponic_bay"]')
    expect(assignBtn.attributes('disabled')).toBeUndefined()
    await assignBtn.trigger('click')

    const emitted = wrapper.emitted('assign-labour')
    expect(emitted).toBeTruthy()
    expect(emitted?.[0]).toEqual(['hydroponic_bay', 8])

    // Draft is cleared after assigning, so the input reflects the prop value again.
    await wrapper.vm.$nextTick()
    expect(wrapper.find('[data-testid="assign-labour-hydroponic_bay"]').attributes('disabled')).toBeDefined()
  })
})
