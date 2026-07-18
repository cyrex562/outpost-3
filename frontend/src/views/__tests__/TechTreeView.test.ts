import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import TechTreeView from '@/views/TechTreeView.vue'
import type { Command } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'

const getTechTree = vi.fn()

vi.mock('@/services/tauriBridge', () => ({
  getTechTree: () => getTechTree(),
}))

const routerPush = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => ({ query: {} }),
}))

const sendCommand = vi.fn<[Command], Promise<GameEvent[]>>()
vi.mock('@/stores/game', () => ({
  useGameStore: () => ({
    sendCommand: (cmd: Command) => sendCommand(cmd),
    busy: false,
  }),
}))

const ALPHA = {
  id: 'alpha',
  name: 'Alpha Tech',
  category: 'industry',
  description: 'First tech.',
  tier: 0,
  cost: 100,
  prerequisites: [],
  state: 'available' as const,
  progress: 0,
  effects: [{ type: 'unlock_building' as const, building_id: 'foundry' }],
  queue_position: null,
}

const BETA = {
  id: 'beta',
  name: 'Beta Tech',
  category: 'science',
  description: 'Second tech.',
  tier: 1,
  cost: 200,
  prerequisites: ['alpha'],
  state: 'locked' as const,
  progress: 0,
  effects: [],
  queue_position: null,
}

const GAMMA_ACTIVE = {
  id: 'gamma',
  name: 'Gamma Tech',
  category: 'industry',
  description: 'Third tech.',
  tier: 0,
  cost: 50,
  prerequisites: [],
  state: 'in_progress' as const,
  progress: 0.4,
  effects: [],
  queue_position: null,
}

const DELTA_QUEUED = {
  id: 'delta',
  name: 'Delta Tech',
  category: 'science',
  description: 'Fourth tech.',
  tier: 0,
  cost: 80,
  prerequisites: [],
  state: 'queued' as const,
  progress: 0,
  effects: [],
  queue_position: 0,
}

/** No active/queued research — used to test the Research action. */
const NODES_IDLE = [ALPHA, BETA]

/** An active project plus a queued follow-on — used to test the queue panel and the Queue action. */
const NODES = [ALPHA, BETA, GAMMA_ACTIVE, DELTA_QUEUED]

beforeEach(() => {
  getTechTree.mockReset()
  sendCommand.mockReset()
  routerPush.mockReset()
  getTechTree.mockResolvedValue(NODES)
  sendCommand.mockResolvedValue([])
})

describe('TechTreeView', () => {
  it('renders every node fetched from getTechTree', async () => {
    const wrapper = mount(TechTreeView)
    await flushPromises()
    expect(wrapper.findAll('.node')).toHaveLength(NODES.length)
  })

  it('filters nodes by category', async () => {
    const wrapper = mount(TechTreeView)
    await flushPromises()
    await wrapper.get('[data-testid="tech-filter-category"]').setValue('industry')
    await flushPromises()
    expect(wrapper.findAll('.node')).toHaveLength(2)
  })

  it('filters nodes by tier', async () => {
    const wrapper = mount(TechTreeView)
    await flushPromises()
    await wrapper.get('[data-testid="tech-filter-tier"]').setValue('1')
    await flushPromises()
    expect(wrapper.findAll('.node')).toHaveLength(1)
  })

  it('shows the active project and queued techs in the queue panel', async () => {
    const wrapper = mount(TechTreeView)
    await flushPromises()
    const panel = wrapper.get('[data-testid="research-queue"]')
    expect(panel.text()).toContain('Gamma Tech')
    expect(panel.text()).toContain('Delta Tech')
  })

  it('sends research_tech when Research is clicked on an available node with no active project', async () => {
    getTechTree.mockResolvedValue(NODES_IDLE)
    const wrapper = mount(TechTreeView)
    await flushPromises()
    await wrapper.get('.node.state-available').trigger('click')
    await wrapper.get('[data-testid="tech-research-btn"]').trigger('click')
    await flushPromises()
    expect(sendCommand).toHaveBeenCalledWith({ kind: 'research_tech', tech_id: 'alpha' })
  })

  it('shows Queue (not Research) for an available node when a project is already active, and sends enqueue_research', async () => {
    const wrapper = mount(TechTreeView)
    await flushPromises()
    await wrapper.get('.node.state-available').trigger('click')
    expect(wrapper.find('[data-testid="tech-research-btn"]').exists()).toBe(false)
    await wrapper.get('[data-testid="tech-enqueue-btn"]').trigger('click')
    await flushPromises()
    expect(sendCommand).toHaveBeenCalledWith({ kind: 'enqueue_research', tech_id: 'alpha' })
  })

  it('sends cancel_research when Cancel is clicked', async () => {
    const wrapper = mount(TechTreeView)
    await flushPromises()
    await wrapper.get('[data-testid="research-queue"] button').trigger('click')
    await flushPromises()
    expect(sendCommand).toHaveBeenCalledWith({ kind: 'cancel_research' })
  })

  it('shows effects for the selected node', async () => {
    const wrapper = mount(TechTreeView)
    await flushPromises()
    await wrapper.get('.node.state-available').trigger('click')
    const effects = wrapper.get('[data-testid="tech-effects"]')
    expect(effects.text()).toContain('foundry')
  })
})
