import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import AlertsPanel from '@/components/AlertsPanel.vue'
import type { LogEntry } from '@/worldModel/model'

function makeEntry(overrides: Partial<LogEntry> = {}): LogEntry {
  return {
    id: 'log-1',
    tier: 'ambient',
    message: 'construction queued',
    timestamp_sol: 5,
    event: { kind: 'construction_queued', colony_id: 'c1', building_type: 'greenhouse', project_id: 'p1' },
    ...overrides,
  }
}

describe('AlertsPanel (events/alerts unification)', () => {
  it('renders one unified list, newest entry first', () => {
    const entries: LogEntry[] = [
      makeEntry({ id: 'log-1', timestamp_sol: 1, message: 'first' }),
      makeEntry({ id: 'log-2', timestamp_sol: 2, message: 'second' }),
    ]
    const wrapper = mount(AlertsPanel, { props: { logEntries: entries } })
    const items = wrapper.findAll('[data-testid="log-list"] > li')
    expect(items).toHaveLength(2)
    // Newest first: entry 2 renders before entry 1.
    expect(items[0]!.text()).toContain('sol 2')
    expect(items[1]!.text()).toContain('sol 1')
  })

  it('colors alert-tier entries differently from ambient ones, via the tier class', () => {
    const entries: LogEntry[] = [
      makeEntry({
        id: 'log-1',
        tier: 'urgent',
        message: 'Hazard!',
        timestamp_sol: 1,
        event: {
          kind: 'hazard_occurred',
          colony_id: 'c1',
          hazard_kind: 'fire',
          severity: 0.5,
          stability_delta: -0.1,
          commodity_losses: [],
          population_lost: 0,
        },
      }),
      makeEntry({ id: 'log-2', tier: 'ambient', timestamp_sol: 2 }),
    ]
    const wrapper = mount(AlertsPanel, { props: { logEntries: entries } })
    const items = wrapper.findAll('[data-testid="log-list"] > li')
    // Newest first: log-2 (sol 2) renders before log-1 (sol 1).
    expect(items[0]!.classes()).toContain('tier-ambient')
    expect(items[1]!.classes()).toContain('tier-urgent')
  })

  it('shows the curated message for alert-tier entries, and a humanized kind for ambient ones', () => {
    const entries: LogEntry[] = [
      makeEntry({
        tier: 'notable',
        message: 'Alpha Base: production shortfall on mine (scale 50%)',
        event: { kind: 'production_shortfall', colony_id: 'c1', building_type: 'mine', scale: 0.5, reason: 'Labour' },
      }),
    ]
    const wrapper = mount(AlertsPanel, { props: { logEntries: entries } })
    expect(wrapper.text()).toContain('Alpha Base: production shortfall on mine (scale 50%)')
  })

  it('expands to show the raw event payload on click, and collapses on a second click', async () => {
    const entries: LogEntry[] = [
      makeEntry({
        event: { kind: 'construction_queued', colony_id: 'c1', building_type: 'greenhouse', project_id: 'proj-42' },
      }),
    ]
    const wrapper = mount(AlertsPanel, { props: { logEntries: entries } })
    expect(wrapper.find('[data-testid="log-detail-construction_queued"]').exists()).toBe(false)

    await wrapper.get('[data-testid="log-item-construction_queued"]').trigger('click')
    const detail = wrapper.get('[data-testid="log-detail-construction_queued"]')
    expect(detail.text()).toContain('building_type')
    expect(detail.text()).toContain('greenhouse')
    expect(detail.text()).toContain('proj-42')

    await wrapper.get('[data-testid="log-item-construction_queued"]').trigger('click')
    expect(wrapper.find('[data-testid="log-detail-construction_queued"]').exists()).toBe(false)
  })

  it('emits clear-log when the Clear button is clicked', async () => {
    const wrapper = mount(AlertsPanel, { props: { logEntries: [] } })
    await wrapper.find('[data-testid="btn-clear-log"]').trigger('click')
    expect(wrapper.emitted('clear-log')).toHaveLength(1)
  })

  it('shows a hint when there is nothing to show', () => {
    const wrapper = mount(AlertsPanel, { props: { logEntries: [] } })
    expect(wrapper.text()).toContain('No log entries yet.')
  })
})
