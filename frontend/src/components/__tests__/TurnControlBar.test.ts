import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import TurnControlBar from '@/components/TurnControlBar.vue'

const sendCommand = vi.fn()
const dismissToast = vi.fn()
const gameStoreMock = {
  busy: false,
  toastMessage: null as string | null,
  sendCommand: (cmd: unknown) => sendCommand(cmd),
  dismissToast: () => dismissToast(),
}
const worldStoreMock = { sol: 7, month: 3 }

vi.mock('@/stores/game', () => ({ useGameStore: () => gameStoreMock }))
vi.mock('@/stores/worldStore', () => ({ useWorldStore: () => worldStoreMock }))

describe('TurnControlBar (UI-rework PR3)', () => {
  beforeEach(() => {
    sendCommand.mockReset()
    dismissToast.mockReset()
    gameStoreMock.busy = false
    gameStoreMock.toastMessage = null
  })

  it('shows the current-turn indicator from the world store', () => {
    const wrapper = mount(TurnControlBar)
    expect(wrapper.get('[data-testid="turn-indicator"]').text()).toContain('Sol 7')
    expect(wrapper.get('[data-testid="turn-indicator"]').text()).toContain('Month 3')
  })

  it('advances the turn on click', async () => {
    const wrapper = mount(TurnControlBar)
    await wrapper.get('[data-testid="btn-advance-turn"]').trigger('click')
    expect(sendCommand).toHaveBeenCalledWith({ kind: 'advance_sol' })
  })

  it('disables Advance Turn while the engine is busy', () => {
    gameStoreMock.busy = true
    const wrapper = mount(TurnControlBar)
    expect(wrapper.get('[data-testid="btn-advance-turn"]').attributes('disabled')).toBeDefined()
  })

  it('shows and dismisses the global event toast', async () => {
    gameStoreMock.toastMessage = 'Colony founded'
    const wrapper = mount(TurnControlBar)
    const toast = wrapper.get('[data-testid="event-toast"]')
    expect(toast.text()).toContain('Colony founded')
    await toast.trigger('click')
    expect(dismissToast).toHaveBeenCalled()
  })
})
