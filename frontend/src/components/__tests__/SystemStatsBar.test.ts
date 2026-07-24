import { describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import SystemStatsBar from '@/components/SystemStatsBar.vue'

const worldStoreMock = {
  researchTotal: 12.5,
  colonies: [
    { id: 'c1', name: 'Alpha', population: 100 },
    { id: 'c2', name: 'Beta', population: 60 },
  ],
  notifications: [{ id: 'n1' }],
}

vi.mock('@/stores/worldStore', () => ({ useWorldStore: () => worldStoreMock }))

describe('SystemStatsBar (UI-rework PR3)', () => {
  it('renders system-wide research, colony count, total population, and alert count', () => {
    const wrapper = mount(SystemStatsBar)
    expect(wrapper.get('[data-testid="stat-research"]').text()).toContain('12.5 RP')
    expect(wrapper.get('[data-testid="stat-colonies"]').text()).toContain('2')
    expect(wrapper.get('[data-testid="stat-population"]').text()).toContain('160')
    expect(wrapper.get('[data-testid="stat-alerts"]').text()).toContain('1')
  })
})
