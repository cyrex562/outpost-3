import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
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

const getInterruptDigest = vi.fn()

vi.mock('@/stores/game', () => ({ useGameStore: () => gameStoreMock }))
vi.mock('@/stores/worldStore', () => ({ useWorldStore: () => worldStoreMock }))
vi.mock('@/services/tauriBridge', () => ({
  getInterruptDigest: () => getInterruptDigest(),
}))

/** A clean fast-forward run: ran to completion, nothing halted it. */
function completedRun(sols: number) {
  return [
    { kind: 'colony_sol_advanced', sol: 8 },
    {
      kind: 'fast_forward_ended',
      sol: 8,
      sols_advanced: sols,
      halted: false,
      halting_reason: null,
    },
  ]
}

/** A run cut short by an interrupt at or above the halt threshold. */
function haltedRun() {
  return [
    { kind: 'colony_sol_advanced', sol: 8 },
    {
      kind: 'fast_forward_ended',
      sol: 8,
      sols_advanced: 1,
      halted: true,
      halting_reason: 'Alpha stability trending to crisis',
    },
  ]
}

const emptyDigest = {
  stopped_at_sol: 8,
  sols_requested: 30,
  halting_message: null,
  halting_tier: null,
  items: [],
}

describe('TurnControlBar (UI-rework PR3)', () => {
  beforeEach(() => {
    sendCommand.mockReset()
    sendCommand.mockResolvedValue([])
    dismissToast.mockReset()
    getInterruptDigest.mockReset()
    getInterruptDigest.mockResolvedValue(emptyDigest)
    gameStoreMock.busy = false
    gameStoreMock.toastMessage = null
  })

  it('shows the current-turn indicator from the world store', () => {
    const wrapper = mount(TurnControlBar)
    expect(wrapper.get('[data-testid="turn-indicator"]').text()).toContain('Sol 7')
    expect(wrapper.get('[data-testid="turn-indicator"]').text()).toContain('Month 3')
  })

  // Advance Turn was removed: play, fast-forward, and it were three triggers
  // for one mechanic, and the slowest play speed already does what it did.
  it('has no Advance Turn button — play and fast-forward are the only clocks', () => {
    const wrapper = mount(TurnControlBar)
    expect(wrapper.find('[data-testid="btn-advance-turn"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="btn-play-pause"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="btn-fast-forward"]').exists()).toBe(true)
  })

  it('disables the remaining clock controls while the engine is busy', () => {
    gameStoreMock.busy = true
    const wrapper = mount(TurnControlBar)
    expect(wrapper.get('[data-testid="btn-fast-forward"]').attributes('disabled')).toBeDefined()
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

// ── Time controls (issue #332 part 3) ────────────────────────────────────────

describe('TurnControlBar time controls (issue #332)', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    sendCommand.mockReset()
    sendCommand.mockResolvedValue(completedRun(1))
    getInterruptDigest.mockReset()
    getInterruptDigest.mockResolvedValue(emptyDigest)
    gameStoreMock.busy = false
    gameStoreMock.toastMessage = null
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('sends a fast_forward for the configured span', async () => {
    const wrapper = mount(TurnControlBar)
    await wrapper.get('[data-testid="btn-fast-forward"]').trigger('click')
    expect(sendCommand).toHaveBeenCalledWith({
      kind: 'fast_forward',
      max_sols: 30,
      threshold: 'urgent',
    })
  })

  it('uses the selected halt threshold', async () => {
    const wrapper = mount(TurnControlBar)
    await wrapper.get('[data-testid="select-threshold"]').setValue('blocking')
    await wrapper.get('[data-testid="btn-fast-forward"]').trigger('click')
    expect(sendCommand).toHaveBeenCalledWith(expect.objectContaining({ threshold: 'blocking' }))
  })

  it('does not drive the clock until Play is pressed', async () => {
    mount(TurnControlBar)
    await vi.advanceTimersByTimeAsync(5000)
    expect(sendCommand).not.toHaveBeenCalled()
  })

  it('ticks repeatedly while playing and stops when paused', async () => {
    const wrapper = mount(TurnControlBar)
    const btn = wrapper.get('[data-testid="btn-play-pause"]')

    await btn.trigger('click')
    expect(btn.text()).toContain('Pause')
    await vi.advanceTimersByTimeAsync(3600)
    const whilePlaying = sendCommand.mock.calls.length
    expect(whilePlaying).toBeGreaterThan(1)

    await btn.trigger('click')
    expect(btn.text()).toContain('Play')
    await vi.advanceTimersByTimeAsync(5000)
    expect(sendCommand.mock.calls.length).toBe(whilePlaying)
  })

  it('changing speed mid-run takes effect without waiting out the old interval', async () => {
    const wrapper = mount(TurnControlBar)
    await wrapper.get('[data-testid="btn-play-pause"]').trigger('click')
    // The 5x preset advances 5 sols per tick rather than 1.
    await wrapper.get('[data-testid="btn-speed-2"]').trigger('click')
    await vi.advanceTimersByTimeAsync(700)
    expect(sendCommand).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'fast_forward', max_sols: 5 }),
    )
  })

  it('stops playing and opens the digest when a run halts', async () => {
    getInterruptDigest.mockResolvedValue({
      stopped_at_sol: 8,
      sols_requested: 1,
      halting_message: 'Alpha stability trending to crisis',
      halting_tier: 'urgent',
      items: [
        {
          tier: 'notable',
          message: 'greenhouse construction complete',
          colony_id: null,
          acknowledged: false,
        },
      ],
    })
    sendCommand.mockResolvedValue(haltedRun())

    const wrapper = mount(TurnControlBar)
    const btn = wrapper.get('[data-testid="btn-play-pause"]')
    await btn.trigger('click')
    await vi.advanceTimersByTimeAsync(1300)
    await vi.waitFor(() => {
      expect(wrapper.find('[data-testid="interrupt-digest"]').exists()).toBe(true)
    })

    // The clock must have stopped: a halt hands control back, it does not just
    // annotate a run that keeps going.
    expect(btn.text()).toContain('Play')
    const callsAtHalt = sendCommand.mock.calls.length
    await vi.advanceTimersByTimeAsync(5000)
    expect(sendCommand.mock.calls.length).toBe(callsAtHalt)

    const digest = wrapper.get('[data-testid="interrupt-digest"]')
    expect(digest.text()).toContain('Alpha stability trending to crisis')
    expect(digest.text()).toContain('greenhouse construction complete')
  })

  it('keeps playing when a run completes without halting', async () => {
    const wrapper = mount(TurnControlBar)
    await wrapper.get('[data-testid="btn-play-pause"]').trigger('click')
    await vi.advanceTimersByTimeAsync(2500)
    expect(wrapper.find('[data-testid="interrupt-digest"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="btn-play-pause"]').text()).toContain('Pause')
  })

  // `gameStore.sendCommand` catches its own errors and resolves with `[]`; it
  // does not reject. A test that mocks a rejection therefore proves nothing
  // about the real failure path — the clock has to stop on the empty resolve.
  it('stops the clock when the engine rejects the command (store resolves [])', async () => {
    sendCommand.mockResolvedValue([])
    const wrapper = mount(TurnControlBar)
    const btn = wrapper.get('[data-testid="btn-play-pause"]')
    await btn.trigger('click')
    await vi.advanceTimersByTimeAsync(1300)
    await vi.waitFor(() => {
      expect(btn.text()).toContain('Play')
    })
    const calls = sendCommand.mock.calls.length
    expect(calls).toBeGreaterThan(0)
    // A spinning timer would keep hammering a command that cannot succeed.
    await vi.advanceTimersByTimeAsync(5000)
    expect(sendCommand.mock.calls.length).toBe(calls)
  })

  // Defensive: a future store that does throw must not spin either.
  it('stops the clock if the store throws outright', async () => {
    sendCommand.mockRejectedValue(new Error('engine said no'))
    const wrapper = mount(TurnControlBar)
    const btn = wrapper.get('[data-testid="btn-play-pause"]')
    await btn.trigger('click')
    await vi.advanceTimersByTimeAsync(1300)
    await vi.waitFor(() => {
      expect(btn.text()).toContain('Play')
    })
    const calls = sendCommand.mock.calls.length
    await vi.advanceTimersByTimeAsync(5000)
    expect(sendCommand.mock.calls.length).toBe(calls)
  })

  it('closes the digest on demand', async () => {
    sendCommand.mockResolvedValue(haltedRun())
    const wrapper = mount(TurnControlBar)
    await wrapper.get('[data-testid="btn-fast-forward"]').trigger('click')
    await vi.waitFor(() => {
      expect(wrapper.find('[data-testid="interrupt-digest"]').exists()).toBe(true)
    })
    await wrapper.get('[data-testid="btn-close-digest"]').trigger('click')
    expect(wrapper.find('[data-testid="interrupt-digest"]').exists()).toBe(false)
  })

  it('does not stack commands while one is in flight', async () => {
    gameStoreMock.busy = true
    const wrapper = mount(TurnControlBar)
    await wrapper.get('[data-testid="btn-play-pause"]').trigger('click')
    await vi.advanceTimersByTimeAsync(5000)
    expect(sendCommand).not.toHaveBeenCalled()
  })
})
