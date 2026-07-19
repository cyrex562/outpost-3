import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import MainMenuView from '@/views/MainMenuView.vue'
import type { SnapshotPayload } from '@/services/tauriBridge'

const routerPush = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush }),
}))

const bootstrap = vi.fn<unknown[], Promise<SnapshotPayload>>()
const listCustomPresets = vi.fn()
const listSaves = vi.fn()

vi.mock('@/services/tauriBridge', () => ({
  isTauri: true,
  bootstrap: (...args: unknown[]) => bootstrap(...args),
  listSaves: (...args: unknown[]) => listSaves(...args),
  loadGame: vi.fn(),
  listCustomPresets: () => listCustomPresets(),
  deleteCustomPreset: vi.fn(),
  exitApp: vi.fn(),
}))

vi.mock('@/stores/worldStore', () => ({
  useWorldStore: () => ({ hydrate: vi.fn() }),
}))

describe('MainMenuView system-generation sliders (playtest feedback round 3, Tauri desktop New Game panel)', () => {
  beforeEach(() => {
    routerPush.mockReset()
    bootstrap.mockReset()
    listCustomPresets.mockReset()
    listCustomPresets.mockResolvedValue([])
    bootstrap.mockResolvedValue({ sol: 0, month: 0, colonies: [], research_total: 0 })
  })

  it('renders a habitable-zone, inner-planet-count, and abundance slider with sane defaults', async () => {
    const wrapper = mount(MainMenuView)
    await wrapper.get('[data-testid="btn-new-game"]').trigger('click')

    expect((wrapper.get('[data-testid="hz-center-slider"]').element as HTMLInputElement).value).toBe('1')
    expect((wrapper.get('[data-testid="inner-planet-count-slider"]').element as HTMLInputElement).value).toBe('3')
    expect((wrapper.get('[data-testid="abundance-slider"]').element as HTMLInputElement).value).toBe('1')
  })

  it('sends the independent system seed and generation sliders through bootstrap()', async () => {
    const wrapper = mount(MainMenuView)
    await wrapper.get('[data-testid="btn-new-game"]').trigger('click')

    await wrapper.get('[data-testid="hz-center-slider"]').setValue('1.5')
    await wrapper.get('[data-testid="inner-planet-count-slider"]').setValue('5')
    await wrapper.get('[data-testid="abundance-slider"]').setValue('2')

    await wrapper.get('[data-testid="btn-start"]').trigger('click')
    await new Promise((r) => setTimeout(r, 0))

    expect(bootstrap).toHaveBeenCalledTimes(1)
    const args = bootstrap.mock.calls[0]
    // bootstrap(contentDir, planetSeed, difficulty, ...customDifficultyArgs, systemSeed, genParams)
    const systemSeedArg = args[7]
    const genParams = args[8] as {
      habitableZoneCenterAu?: number
      minInnerPlanets?: number
      maxInnerPlanets?: number
      abundanceScalarOverride?: number
    }
    expect(systemSeedArg).toEqual(expect.any(Number))
    expect(genParams.habitableZoneCenterAu).toBeCloseTo(1.5)
    expect(genParams.minInnerPlanets).toBe(5)
    expect(genParams.maxInnerPlanets).toBe(5)
    expect(genParams.abundanceScalarOverride).toBeCloseTo(2)
  })

  it('randomises the system seed independently of the planet seed', async () => {
    const wrapper = mount(MainMenuView)
    await wrapper.get('[data-testid="btn-new-game"]').trigger('click')

    const planetSeedBefore = (wrapper.get('[data-testid="planet-seed-input"]').element as HTMLInputElement).value
    const systemSeedBefore = (wrapper.get('[data-testid="system-seed-input"]').element as HTMLInputElement).value

    await wrapper.get('[data-testid="randomise-system-seed"]').trigger('click')

    const planetSeedAfter = (wrapper.get('[data-testid="planet-seed-input"]').element as HTMLInputElement).value
    const systemSeedAfter = (wrapper.get('[data-testid="system-seed-input"]').element as HTMLInputElement).value

    expect(planetSeedAfter).toBe(planetSeedBefore)
    expect(systemSeedAfter).not.toBe(systemSeedBefore)
  })
})
