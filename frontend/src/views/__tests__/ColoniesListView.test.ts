import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ColoniesListView from '@/views/ColoniesListView.vue'

const routerPush = vi.fn()
vi.mock('vue-router', () => ({ useRouter: () => ({ push: routerPush }) }))

const worldStoreMock = {
  colonies: [] as Array<Record<string, unknown>>,
}
vi.mock('@/stores/worldStore', () => ({ useWorldStore: () => worldStoreMock }))

function makeColony(overrides: Record<string, unknown> = {}) {
  return {
    id: 'c1',
    name: 'Alpha Base',
    population: 240,
    stability: 0.72,
    available_labour: 8,
    commodity_pool: [],
    buildings: ['hq', 'farm'],
    active_construction: ['lab'],
    ...overrides,
  }
}

describe('ColoniesListView (UI-rework PR7)', () => {
  beforeEach(() => {
    routerPush.mockReset()
    worldStoreMock.colonies = []
  })

  it('shows an empty state when no colonies are founded', () => {
    const wrapper = mount(ColoniesListView)
    expect(wrapper.find('[data-testid="no-colonies"]').exists()).toBe(true)
  })

  it('renders a card with a vital-stat summary for each colony', () => {
    worldStoreMock.colonies = [makeColony(), makeColony({ id: 'c2', name: 'Beta' })]
    const wrapper = mount(ColoniesListView)
    const card = wrapper.get('[data-testid="colony-card-c1"]')
    expect(card.text()).toContain('Alpha Base')
    expect(card.get('[data-testid="summary-population"]').text()).toBe('240')
    expect(card.text()).toContain('72%') // stability
    expect(wrapper.find('[data-testid="colony-card-c2"]').exists()).toBe(true)
  })

  it('opens a colony dashboard when its card is clicked', async () => {
    worldStoreMock.colonies = [makeColony()]
    const wrapper = mount(ColoniesListView)
    await wrapper.get('[data-testid="colony-card-c1"]').trigger('click')
    expect(routerPush).toHaveBeenCalledWith({ name: 'colony', params: { colonyId: 'c1' } })
  })
})
