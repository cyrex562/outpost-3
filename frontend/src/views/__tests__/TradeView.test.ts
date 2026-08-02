import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import TradeView from '@/views/TradeView.vue'
import type { TradeRoute } from '@/services/tauriBridge'

const getTradeRoutes = vi.fn<[], Promise<TradeRoute[]>>()
vi.mock('@/services/tauriBridge', () => ({
  getTradeRoutes: () => getTradeRoutes(),
}))

const sendCommand = vi.fn()
const gameStoreMock = {
  sendCommand: (cmd: unknown) => sendCommand(cmd),
  toastMessage: null as string | null,
}
vi.mock('@/stores/game', () => ({
  useGameStore: () => gameStoreMock,
}))

const worldStoreMock = {
  colonies: [] as Array<{ id: string; name: string }>,
}
vi.mock('@/stores/worldStore', () => ({ useWorldStore: () => worldStoreMock }))

function makeRoute(overrides: Partial<TradeRoute> = {}): TradeRoute {
  return {
    id: 'route-1',
    colony_a: 'colony-1',
    colony_b: 'colony-2',
    throughput_cap: 20,
    transit_sols: 1,
    ...overrides,
  }
}

describe('TradeView (issue #363: cross-body trade route UI)', () => {
  beforeEach(() => {
    getTradeRoutes.mockReset()
    sendCommand.mockReset()
    gameStoreMock.toastMessage = null
    worldStoreMock.colonies = [
      { id: 'colony-1', name: 'Alpha' },
      { id: 'colony-2', name: 'Beta' },
    ]
  })

  it('lists existing trade routes with resolved colony names', async () => {
    getTradeRoutes.mockResolvedValueOnce([makeRoute()])
    const wrapper = mount(TradeView)
    await flushPromises()

    const row = wrapper.get('[data-testid="route-row-route-1"]')
    expect(row.text()).toContain('Alpha')
    expect(row.text()).toContain('Beta')
    expect(row.text()).toContain('20')
  })

  it('shows an empty state when there are no routes', async () => {
    getTradeRoutes.mockResolvedValueOnce([])
    const wrapper = mount(TradeView)
    await flushPromises()

    expect(wrapper.find('[data-testid="no-routes"]').exists()).toBe(true)
  })

  it('adds a trade route between two picked colonies, including cross-body pairs', async () => {
    getTradeRoutes.mockResolvedValue([])
    sendCommand.mockResolvedValueOnce([{ kind: 'trade_route_added' }])
    const wrapper = mount(TradeView)
    await flushPromises()

    await wrapper.get('[data-testid="colony-a-select"]').setValue('colony-1')
    await wrapper.get('[data-testid="colony-b-select"]').setValue('colony-2')
    await wrapper.get('[data-testid="throughput-cap-input"]').setValue(35)
    await wrapper.get('[data-testid="btn-add-route"]').trigger('click')
    await flushPromises()

    expect(sendCommand).toHaveBeenCalledWith({
      kind: 'add_trade_route',
      colony_a: 'colony-1',
      colony_b: 'colony-2',
      throughput_cap: 35,
    })
    // Refreshes the list after adding (initial load + post-add refresh).
    expect(getTradeRoutes).toHaveBeenCalledTimes(2)
  })

  it('keeps the Add Route button disabled until two distinct colonies are picked', async () => {
    getTradeRoutes.mockResolvedValue([])
    const wrapper = mount(TradeView)
    await flushPromises()

    expect(wrapper.get('[data-testid="btn-add-route"]').attributes('disabled')).toBeDefined()

    await wrapper.get('[data-testid="colony-a-select"]').setValue('colony-1')
    expect(wrapper.get('[data-testid="btn-add-route"]').attributes('disabled')).toBeDefined()

    await wrapper.get('[data-testid="colony-b-select"]').setValue('colony-1')
    // Same colony picked twice — still disabled.
    expect(wrapper.get('[data-testid="btn-add-route"]').attributes('disabled')).toBeDefined()
  })

  it('removes a route from the list', async () => {
    getTradeRoutes.mockResolvedValue([makeRoute()])
    sendCommand.mockResolvedValueOnce([{ kind: 'trade_route_removed' }])
    const wrapper = mount(TradeView)
    await flushPromises()

    await wrapper.get('[data-testid="route-row-route-1"] .btn.danger').trigger('click')
    await flushPromises()

    expect(sendCommand).toHaveBeenCalledWith({
      kind: 'remove_trade_route',
      route_id: 'route-1',
    })
    expect(getTradeRoutes).toHaveBeenCalledTimes(2)
  })

  it('surfaces the rejection reason when adding a route is refused (empty events)', async () => {
    getTradeRoutes.mockResolvedValue([])
    sendCommand.mockResolvedValueOnce([]) // engine rejected the command
    gameStoreMock.toastMessage = 'Error: throughput_cap must be >= 0'
    const wrapper = mount(TradeView)
    await flushPromises()

    await wrapper.get('[data-testid="colony-a-select"]').setValue('colony-1')
    await wrapper.get('[data-testid="colony-b-select"]').setValue('colony-2')
    await wrapper.get('[data-testid="btn-add-route"]').trigger('click')
    await flushPromises()

    expect(wrapper.get('[data-testid="trade-error"]').text()).toContain('throughput_cap must be >= 0')
    // A rejected add did not refresh (only the initial load ran).
    expect(getTradeRoutes).toHaveBeenCalledTimes(1)
  })

  it('surfaces an error when the trade route list fails to load', async () => {
    getTradeRoutes.mockRejectedValueOnce(new Error('engine not initialised'))
    const wrapper = mount(TradeView)
    await flushPromises()

    expect(wrapper.get('[data-testid="trade-error"]').text()).toContain('engine not initialised')
  })
})
