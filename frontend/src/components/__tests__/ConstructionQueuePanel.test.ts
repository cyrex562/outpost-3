import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ConstructionQueuePanel from '@/components/ConstructionQueuePanel.vue'
import type { ConstructionQueueRow } from '@/types/screen'

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

describe('ConstructionQueuePanel (UI-rework PR5: in-progress list only)', () => {
  it('emits cancel with the project id when clicked', async () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: { queue: [makeQueueRow({ project_id: 'proj-42' })], cancelingIds: new Set<string>() },
    })
    await wrapper.find('[data-testid="btn-cancel-proj-42"]').trigger('click')
    expect(wrapper.emitted('cancel')).toEqual([['proj-42']])
  })

  it('disables and relabels the cancel button while a cancel is in flight', () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: { queue: [makeQueueRow({ project_id: 'proj-42' })], cancelingIds: new Set(['proj-42']) },
    })
    const btn = wrapper.find('[data-testid="btn-cancel-proj-42"]')
    expect(btn.attributes('disabled')).toBeDefined()
    expect(btn.text()).toBe('Cancelling…')
  })

  it('shows a hint instead of a list when nothing is under construction', () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: { queue: [], cancelingIds: new Set<string>() },
    })
    expect(wrapper.find('[data-testid="construction-queue-list"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('Nothing under construction')
  })

  it('emits open-build when the Build button is clicked', async () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: { queue: [], cancelingIds: new Set<string>() },
    })
    await wrapper.find('[data-testid="btn-open-build"]').trigger('click')
    expect(wrapper.emitted('open-build')).toHaveLength(1)
  })

  it('no longer renders the build catalog inline (moved to BuildDialog)', () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: { queue: [], cancelingIds: new Set<string>() },
    })
    expect(wrapper.find('[data-testid="build-card-smelter"]').exists()).toBe(false)
  })
})

describe('ConstructionQueuePanel empty-queue collapse (issue #339)', () => {
  it('marks the panel collapsed when the queue is empty', () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: { queue: [], cancelingIds: new Set<string>() },
    })
    const panel = wrapper.get('[data-testid="construction-queue-panel"]')
    expect(panel.attributes('data-collapsed')).toBe('true')
    expect(panel.classes()).toContain('panel--collapsed')
  })

  it('marks the panel collapsed when the queue has not loaded yet (null)', () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: { queue: null, cancelingIds: new Set<string>() },
    })
    expect(wrapper.get('[data-testid="construction-queue-panel"]').attributes('data-collapsed')).toBe(
      'true',
    )
  })

  it('is not collapsed once something is queued', () => {
    const wrapper = mount(ConstructionQueuePanel, {
      props: { queue: [makeQueueRow({ project_id: 'proj-1' })], cancelingIds: new Set<string>() },
    })
    const panel = wrapper.get('[data-testid="construction-queue-panel"]')
    expect(panel.attributes('data-collapsed')).toBe('false')
    expect(panel.classes()).not.toContain('panel--collapsed')
  })
})
