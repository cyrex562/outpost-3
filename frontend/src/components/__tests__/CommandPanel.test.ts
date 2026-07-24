import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import CommandPanel from '@/components/CommandPanel.vue'

const sendCommand = vi.fn()
const dismissToast = vi.fn()
const gameStoreMock = {
  selectedColonyId: 'colony-1' as string | null,
  busy: false,
  toastMessage: null as string | null,
  sendCommand: (cmd: unknown) => sendCommand(cmd),
  dismissToast: () => dismissToast(),
}
vi.mock('@/stores/game', () => ({
  useGameStore: () => gameStoreMock,
}))

describe('CommandPanel (UI-rework: obsolete controls removed)', () => {
  beforeEach(() => {
    sendCommand.mockReset()
    dismissToast.mockReset()
    gameStoreMock.busy = false
    gameStoreMock.toastMessage = null
    gameStoreMock.selectedColonyId = 'colony-1'
  })

  it('advances the turn', async () => {
    const wrapper = mount(CommandPanel)
    await wrapper.get('[data-testid="btn-advance-turn"]').trigger('click')
    expect(sendCommand).toHaveBeenCalledWith({ kind: 'advance_sol' })
  })

  it('disables Advance Turn while the engine is busy', () => {
    gameStoreMock.busy = true
    const wrapper = mount(CommandPanel)
    expect(wrapper.get('[data-testid="btn-advance-turn"]').attributes('disabled')).toBeDefined()
  })

  it('no longer renders the obsolete Found Colony / Set Directive / Research Tech controls', () => {
    const wrapper = mount(CommandPanel)
    // Founding moved to the system/surface map; research to the tech tree.
    expect(wrapper.find('[data-testid="btn-found-colony"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="btn-set-directive"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="btn-research-tech"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="set-directive-dialog"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="research-tech-dialog"]').exists()).toBe(false)
  })

  it('shows and dismisses the event toast', async () => {
    gameStoreMock.toastMessage = 'Colony founded'
    const wrapper = mount(CommandPanel)
    const toast = wrapper.get('[data-testid="event-toast"]')
    expect(toast.text()).toContain('Colony founded')
    await toast.trigger('click')
    expect(dismissToast).toHaveBeenCalled()
  })
})
