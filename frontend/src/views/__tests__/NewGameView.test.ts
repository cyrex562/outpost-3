import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import NewGameView from '@/views/NewGameView.vue'
import type { ClientCommandMessage, ClientMessage } from '@/types/api'

const routerPush = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
}))

const send = vi.fn<[ClientMessage], void>()
vi.mock('@/composables/useGameSocket', () => ({
  useGameSocket: () => ({ send }),
}))

vi.mock('@/stores/worldStore', () => ({
  useWorldStore: () => ({ isConnected: true }),
}))

describe('NewGameView system-generation sliders (playtest feedback round 3)', () => {
  beforeEach(() => {
    routerPush.mockReset()
    send.mockReset()
  })

  it('renders a habitable-zone, inner-planet-count, and abundance slider with sane defaults', () => {
    const wrapper = mount(NewGameView)
    expect((wrapper.get('[data-testid="hz-center-slider"]').element as HTMLInputElement).value).toBe('1')
    expect((wrapper.get('[data-testid="inner-planet-count-slider"]').element as HTMLInputElement).value).toBe('3')
    expect((wrapper.get('[data-testid="abundance-slider"]').element as HTMLInputElement).value).toBe('1')
  })

  it('sends the independent system seed and generation sliders in the new_game command', async () => {
    const wrapper = mount(NewGameView)

    await wrapper.get('[data-testid="hz-center-slider"]').setValue('1.5')
    await wrapper.get('[data-testid="inner-planet-count-slider"]').setValue('5')
    await wrapper.get('[data-testid="abundance-slider"]').setValue('2')

    await wrapper.get('[data-testid="start-game-btn"]').trigger('click')

    expect(send).toHaveBeenCalledTimes(1)
    const msg = send.mock.calls[0][0] as ClientCommandMessage
    expect(msg.type).toBe('command')
    const cmd = msg.command as Extract<ClientCommandMessage['command'], { kind: 'new_game' }>
    expect(cmd.kind).toBe('new_game')
    expect(cmd.system_seed).toEqual(expect.any(Number))
    expect(cmd.habitable_zone_center_au).toBeCloseTo(1.5)
    expect(cmd.min_inner_planets).toBe(5)
    expect(cmd.max_inner_planets).toBe(5)
    expect(cmd.abundance_scalar).toBeCloseTo(2)
  })

  it('randomises the system seed independently of the planet seed', async () => {
    const wrapper = mount(NewGameView)
    const planetSeedBefore = (wrapper.get('[data-testid="planet-seed-input"]').element as HTMLInputElement).value
    const systemSeedBefore = (wrapper.get('[data-testid="system-seed-input"]').element as HTMLInputElement).value

    await wrapper.get('[data-testid="randomise-system-seed"]').trigger('click')

    const planetSeedAfter = (wrapper.get('[data-testid="planet-seed-input"]').element as HTMLInputElement).value
    const systemSeedAfter = (wrapper.get('[data-testid="system-seed-input"]').element as HTMLInputElement).value

    expect(planetSeedAfter).toBe(planetSeedBefore)
    expect(systemSeedAfter).not.toBe(systemSeedBefore)
  })
})
