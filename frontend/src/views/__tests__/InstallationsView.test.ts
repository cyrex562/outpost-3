import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import InstallationsView from '@/views/InstallationsView.vue'
import { useWorldStore } from '@/stores/worldStore'
import type { Outpost } from '@/services/tauriBridge'
import type { Command } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'

const listOutposts = vi.fn<[], Promise<Outpost[]>>()

vi.mock('@/services/tauriBridge', () => ({
  isTauri: false,
  listOutposts: () => listOutposts(),
}))

const sendCommand = vi.fn<[Command], Promise<GameEvent[]>>()
// A single shared object (not a fresh literal per call) so the component's
// `gameStore.toastMessage` reads see whatever a test sets on it, mirroring
// the real Pinia store.
const gameStoreMock = {
  sendCommand: (cmd: Command) => sendCommand(cmd),
  toastMessage: null as string | null,
}
vi.mock('@/stores/game', () => ({
  useGameStore: () => gameStoreMock,
}))

function makeOutpost(overrides: Partial<Outpost>): Outpost {
  return {
    id: 'outpost-1',
    name: 'Forward Base',
    parent_colony_id: 'colony-1',
    body_id: 'body-1',
    body_name: 'Luna',
    slot_capacity: 3,
    slots_used: 1,
    buildings: ['excavation_rig'],
    pool: [],
    ...overrides,
  }
}

describe('InstallationsView (navigation rework #7 phase 3: system-wide installations list)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    listOutposts.mockReset()
    sendCommand.mockReset()
    gameStoreMock.toastMessage = null
  })

  it('lists every outpost system-wide, unfiltered by colony', async () => {
    listOutposts.mockResolvedValueOnce([
      makeOutpost({ id: 'outpost-1', name: 'Forward Base', parent_colony_id: 'colony-1' }),
      makeOutpost({ id: 'outpost-2', name: 'Rear Base', parent_colony_id: 'colony-2' }),
    ])
    const wrapper = mount(InstallationsView)
    await flushPromises()

    expect(wrapper.find('[data-testid="installation-outpost-1"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="installation-outpost-2"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('2 outposts system-wide')
  })

  it('resolves the owning colony name from worldStore', async () => {
    const worldStore = useWorldStore()
    worldStore.world.colonies = {
      'colony-1': {
        id: 'colony-1',
        name: 'Alpha Base',
        population: 100,
        stability: 0.9,
        available_labour: 5,
        buildings: [],
        active_projects: [],
        commodity_pool: [],
        active_construction: [],
      },
    }
    listOutposts.mockResolvedValueOnce([makeOutpost({ parent_colony_id: 'colony-1' })])
    const wrapper = mount(InstallationsView)
    await flushPromises()

    expect(wrapper.text()).toContain('Alpha Base')
  })

  it('shows the empty state when no outposts exist', async () => {
    listOutposts.mockResolvedValueOnce([])
    const wrapper = mount(InstallationsView)
    await flushPromises()

    expect(wrapper.text()).toContain('No outposts established anywhere yet.')
  })

  it('decommissions an outpost and refreshes the list on success', async () => {
    listOutposts.mockResolvedValueOnce([makeOutpost({ id: 'outpost-1' })])
    listOutposts.mockResolvedValueOnce([])
    sendCommand.mockResolvedValueOnce([{ kind: 'outpost_decommissioned' } as unknown as never])
    const wrapper = mount(InstallationsView)
    await flushPromises()

    await wrapper.get('[data-testid="installation-outpost-1"] button').trigger('click')
    await flushPromises()

    expect(sendCommand).toHaveBeenCalledWith({ kind: 'decommission_outpost', outpost_id: 'outpost-1' })
    expect(listOutposts).toHaveBeenCalledTimes(2)
    expect(wrapper.text()).toContain('No outposts established anywhere yet.')
  })

  it('shows an error and keeps the outpost listed when decommission is rejected', async () => {
    listOutposts.mockResolvedValueOnce([makeOutpost({ id: 'outpost-1' })])
    sendCommand.mockResolvedValueOnce([])
    gameStoreMock.toastMessage = 'Error: outpost has active projects.'
    const wrapper = mount(InstallationsView)
    await flushPromises()

    await wrapper.get('[data-testid="installation-outpost-1"] button').trigger('click')
    await flushPromises()

    expect(listOutposts).toHaveBeenCalledTimes(1)
    expect(wrapper.find('[data-testid="installation-outpost-1"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Error: outpost has active projects.')
  })
})
