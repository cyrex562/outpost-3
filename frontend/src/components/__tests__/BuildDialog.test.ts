import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import BuildDialog from '@/components/BuildDialog.vue'
import type { BuildingOption } from '@/services/tauriBridge'

function makeBuildingOption(overrides: Partial<BuildingOption>): BuildingOption {
  return {
    id: 'smelter',
    name: 'Smelter',
    description: '',
    category: 'Industry',
    slot_cost: 2,
    labor_per_turn: 3,
    construction_turns: 5,
    construction_cost: [],
    tech_prerequisite: null,
    ...overrides,
  }
}

function mountDialog(catalog: BuildingOption[], disabledReason: (b: BuildingOption) => string | null = () => null) {
  return mount(BuildDialog, {
    props: { catalog, disabledReason, slotsAvailable: 3, busy: false },
  })
}

describe('BuildDialog (UI-rework PR5)', () => {
  it('queues a building with quantity 1 by default', async () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option])
    await wrapper.find('[data-testid="btn-queue-research_lab"]').trigger('click')
    expect(wrapper.emitted('queue')).toEqual([[option, 1]])
  })

  it('queues the chosen quantity', async () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option])
    await wrapper.find('[data-testid="qty-research_lab"]').setValue('4')
    await wrapper.find('[data-testid="btn-queue-research_lab"]').trigger('click')
    expect(wrapper.emitted('queue')).toEqual([[option, 4]])
  })

  it('clamps a blank/zero quantity up to 1', async () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option])
    await wrapper.find('[data-testid="qty-research_lab"]').setValue('0')
    await wrapper.find('[data-testid="btn-queue-research_lab"]').trigger('click')
    expect(wrapper.emitted('queue')).toEqual([[option, 1]])
  })

  it('disables Queue and shows the reason when a building is gated', () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option], () => 'Requires: basic_metallurgy')
    expect(wrapper.find('[data-testid="btn-queue-research_lab"]').attributes('disabled')).toBeDefined()
    expect(wrapper.find('[data-testid="build-card-reason-research_lab"]').text()).toBe(
      'Requires: basic_metallurgy',
    )
  })

  it('emits close on the close button', async () => {
    const wrapper = mountDialog([])
    await wrapper.find('[data-testid="btn-close-build"]').trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
  })
})
