import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import AlertsPanel from '@/components/AlertsPanel.vue'
import type { Notification } from '@/worldModel/model'
import type { ServerEvent } from '@/types/events'

describe('AlertsPanel (issue #169)', () => {
  it('renders notifications and event log entries', () => {
    const notifications: Notification[] = [
      { id: 'n1', tier: 'urgent', message: 'Stability critical', colony_id: 'c1', timestamp_sol: 5 },
    ]
    const eventLog = [{ kind: 'construction_queued' } as ServerEvent]
    const wrapper = mount(AlertsPanel, { props: { notifications, eventLog } })

    expect(wrapper.text()).toContain('Stability critical')
    expect(wrapper.find('[data-testid="event-log-list"]').text()).toContain('construction queued')
  })

  it('emits clear-log when the Clear button is clicked', async () => {
    const wrapper = mount(AlertsPanel, { props: { notifications: [], eventLog: [] } })
    await wrapper.find('[data-testid="btn-clear-event-log"]').trigger('click')
    expect(wrapper.emitted('clear-log')).toHaveLength(1)
  })

  it('shows hints when there is nothing to show', () => {
    const wrapper = mount(AlertsPanel, { props: { notifications: [], eventLog: [] } })
    expect(wrapper.text()).toContain('No alerts yet.')
    expect(wrapper.text()).toContain('No events yet.')
  })
})
