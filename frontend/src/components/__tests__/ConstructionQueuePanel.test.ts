import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ConstructionQueuePanel from '@/components/ConstructionQueuePanel.vue'
import type { ConstructionQueueRow } from '@/types/screen'
import type { BuildingOption } from '@/services/tauriBridge'

function makeQueueRow(overrides: Partial<ConstructionQueueRow>): ConstructionQueueRow {
  return {
    project_id: 'proj-1',
    building_type: 'smelter',
    turns_completed: 2,
    turns_total: 5,
    slot_cost: 2,
    ...overrides,
  }
}

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

describe('ConstructionQueuePanel cancel (issue #169)', () => {
  it('emits cancel with the project id when clicked', async () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: {
        queue: [makeQueueRow({ project_id: 'proj-42' })],
        catalog: [],
        disabledReason: () => null,
        slotsAvailable: 3,
        queueBusy: false,
        cancelingIds: new Set<string>(),
      },
    })
    await wrapper.find('[data-testid="btn-cancel-proj-42"]').trigger('click')
    expect(wrapper.emitted('cancel')).toEqual([['proj-42']])
  })

  it('disables and relabels the cancel button while a cancel is in flight', () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: {
        queue: [makeQueueRow({ project_id: 'proj-42' })],
        catalog: [],
        disabledReason: () => null,
        slotsAvailable: 3,
        queueBusy: false,
        cancelingIds: new Set(['proj-42']),
      },
    })
    const btn = wrapper.find('[data-testid="btn-cancel-proj-42"]')
    expect(btn.attributes('disabled')).toBeDefined()
    expect(btn.text()).toBe('Cancelling…')
  })

  it('shows a hint instead of a list when the queue is empty', () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: {
        queue: [],
        catalog: [],
        disabledReason: () => null,
        slotsAvailable: 3,
        queueBusy: false,
        cancelingIds: new Set<string>(),
      },
    })
    expect(wrapper.find('[data-testid="construction-queue-list"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('No projects in queue.')
  })
})

describe('ConstructionQueuePanel build catalog', () => {
  it('emits queue with the building option when Queue is clicked', async () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mount(ConstructionQueuePanel, {
      props: {
        queue: null,
        catalog: [option],
        disabledReason: () => null,
        slotsAvailable: 3,
        queueBusy: false,
        cancelingIds: new Set<string>(),
      },
    })
    await wrapper.find('[data-testid="btn-queue-research_lab"]').trigger('click')
    expect(wrapper.emitted('queue')).toEqual([[option]])
  })

  it('disables the Queue button and shows the reason when disabledReason returns non-null', () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mount(ConstructionQueuePanel, {
      props: {
        queue: null,
        catalog: [option],
        disabledReason: () => 'Requires: basic_metallurgy',
        slotsAvailable: 3,
        queueBusy: false,
        cancelingIds: new Set<string>(),
      },
    })
    expect(wrapper.find('[data-testid="btn-queue-research_lab"]').attributes('disabled')).toBeDefined()
    expect(wrapper.find('[data-testid="build-card-reason-research_lab"]').text()).toBe(
      'Requires: basic_metallurgy',
    )
  })
})
