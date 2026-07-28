import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import BalanceTuningPanel from '@/components/BalanceTuningPanel.vue'
import type { BalanceScalar } from '@/services/tauriBridge'

const getBalanceScalars = vi.fn<[], Promise<BalanceScalar[]>>()
vi.mock('@/services/tauriBridge', () => ({
  getBalanceScalars: () => getBalanceScalars(),
}))

const sendCommand = vi.fn()
vi.mock('@/stores/game', () => ({
  useGameStore: () => ({ sendCommand }),
}))

function rows(overrides: Partial<BalanceScalar>[] = []): BalanceScalar[] {
  const base: BalanceScalar[] = [
    { quantity: 'resource_consumption', value: 1, min: 0.01, max: 100 },
    { quantity: 'hazard_probability', value: 1, min: 0.01, max: 100 },
  ]
  return base.map((r, i) => ({ ...r, ...(overrides[i] ?? {}) }))
}

describe('BalanceTuningPanel (live balance editing for playtesting)', () => {
  beforeEach(() => {
    getBalanceScalars.mockReset()
    sendCommand.mockReset()
    sendCommand.mockResolvedValue([])
  })

  it('renders a dial for every scalar the engine reports', async () => {
    getBalanceScalars.mockResolvedValue(rows())
    const wrapper = mount(BalanceTuningPanel)
    await flushPromises()

    expect(wrapper.find('[data-testid="dial-resource_consumption"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="dial-hazard_probability"]').exists()).toBe(true)
  })

  it('sends set_balance_scalar for the edited dial only', async () => {
    getBalanceScalars.mockResolvedValue(rows())
    const wrapper = mount(BalanceTuningPanel)
    await flushPromises()

    const input = wrapper.get('[data-testid="number-resource_consumption"]')
    ;(input.element as HTMLInputElement).value = '2.5'
    await input.trigger('change')
    await flushPromises()

    expect(sendCommand).toHaveBeenCalledTimes(1)
    expect(sendCommand).toHaveBeenCalledWith({
      kind: 'set_balance_scalar',
      quantity: 'resource_consumption',
      value: 2.5,
    })
  })

  // The engine clamps, so the panel must re-read rather than display the raw
  // input — otherwise a value past the bound would show a number the sim isn't
  // actually using.
  it('re-reads from the engine after applying, so clamping is visible', async () => {
    getBalanceScalars.mockResolvedValueOnce(rows()).mockResolvedValueOnce(rows([{ value: 100 }]))
    const wrapper = mount(BalanceTuningPanel)
    await flushPromises()

    const input = wrapper.get('[data-testid="number-resource_consumption"]')
    ;(input.element as HTMLInputElement).value = '99999'
    await input.trigger('change')
    await flushPromises()

    expect(getBalanceScalars).toHaveBeenCalledTimes(2)
    expect(wrapper.get('[data-testid="dial-value-resource_consumption"]').text()).toBe('×100.00')
  })

  it('marks a modified dial and lets it be reset to 1.0', async () => {
    getBalanceScalars.mockResolvedValue(rows([{ value: 2 }]))
    const wrapper = mount(BalanceTuningPanel)
    await flushPromises()

    const value = wrapper.get('[data-testid="dial-value-resource_consumption"]')
    expect(value.classes()).toContain('modified')

    await wrapper.get('[data-testid="reset-resource_consumption"]').trigger('click')
    await flushPromises()
    expect(sendCommand).toHaveBeenCalledWith({
      kind: 'set_balance_scalar',
      quantity: 'resource_consumption',
      value: 1,
    })
  })

  it('disables reset on a dial already at its default', async () => {
    getBalanceScalars.mockResolvedValue(rows())
    const wrapper = mount(BalanceTuningPanel)
    await flushPromises()

    const reset = wrapper.get('[data-testid="reset-hazard_probability"]')
    expect((reset.element as HTMLButtonElement).disabled).toBe(true)
  })

  it('surfaces a load failure instead of rendering an empty panel', async () => {
    getBalanceScalars.mockRejectedValueOnce(new Error('engine not initialised'))
    const wrapper = mount(BalanceTuningPanel)
    await flushPromises()

    expect(wrapper.get('[data-testid="balance-error"]').text()).toContain('engine not initialised')
  })
})
