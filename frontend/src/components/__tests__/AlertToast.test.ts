import { describe, expect, it, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import AlertToast from '@/components/AlertToast.vue'
import { useWorldStore } from '@/stores/worldStore'

describe('AlertToast (events/alerts unification)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders nothing when there is no pending alert', () => {
    const wrapper = mount(AlertToast)
    expect(wrapper.find('[data-testid="alert-toast"]').exists()).toBe(false)
  })

  it('shows the alert tier and message when the store has a pending toast', () => {
    const store = useWorldStore()
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'colony_founded', colony_id: 'c1', name: 'Alpha Base', starting_population: 100 },
    })
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'hazard_occurred', colony_id: 'c1', hazard_kind: 'fire', severity: 0.5, stability_delta: -0.1, commodity_losses: [], population_lost: 0 },
    })

    const wrapper = mount(AlertToast)
    const toast = wrapper.get('[data-testid="alert-toast"]')
    expect(toast.classes()).toContain('tier-urgent')
    expect(toast.text()).toContain('urgent')
    expect(toast.text()).toContain('hazard')
  })

  it('dismisses the toast on click', async () => {
    const store = useWorldStore()
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'colony_founded', colony_id: 'c1', name: 'Alpha Base', starting_population: 100 },
    })
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'production_shortfall', colony_id: 'c1', building_type: 'mine', scale: 0.5, reason: 'Labour' },
    })

    const wrapper = mount(AlertToast)
    expect(wrapper.find('[data-testid="alert-toast"]').exists()).toBe(true)
    await wrapper.get('[data-testid="alert-toast"]').trigger('click')
    expect(wrapper.find('[data-testid="alert-toast"]').exists()).toBe(false)
  })
})
