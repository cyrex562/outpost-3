import { beforeEach, describe, expect, it, vi } from 'vitest'
import { usePlanetMapWrap, __resetPlanetMapWrapForTests } from '@/composables/usePlanetMapWrap'

const STORAGE_KEY = 'outpost3.planet-map.wrap'

/**
 * Re-import the module with a fresh registry so its module-scope
 * initialisation (which reads localStorage exactly once) runs again against
 * whatever the test just wrote.
 */
async function freshModule() {
  vi.resetModules()
  return import('@/composables/usePlanetMapWrap')
}

describe('usePlanetMapWrap', () => {
  beforeEach(() => {
    window.localStorage.clear()
    __resetPlanetMapWrapForTests()
  })

  it('defaults to wrapping on', () => {
    expect(usePlanetMapWrap().wrapEnabled.value).toBe(true)
  })

  it('toggles and persists the choice', () => {
    const { wrapEnabled, toggleWrap } = usePlanetMapWrap()

    toggleWrap()
    expect(wrapEnabled.value).toBe(false)
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('false')

    toggleWrap()
    expect(wrapEnabled.value).toBe(true)
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('true')
  })

  it('shares one value across every caller, so two maps never disagree', () => {
    const a = usePlanetMapWrap()
    const b = usePlanetMapWrap()

    a.toggleWrap()
    expect(b.wrapEnabled.value).toBe(false)
  })

  it('restores a persisted "off" on reload', async () => {
    window.localStorage.setItem(STORAGE_KEY, 'false')
    const mod = await freshModule()
    expect(mod.usePlanetMapWrap().wrapEnabled.value).toBe(false)
  })

  it('falls back to the default when the stored value is corrupt', async () => {
    window.localStorage.setItem(STORAGE_KEY, 'not-a-boolean')
    const mod = await freshModule()
    expect(mod.usePlanetMapWrap().wrapEnabled.value).toBe(true)
  })

  it('still works when localStorage is unavailable', async () => {
    const getItem = vi
      .spyOn(Storage.prototype, 'getItem')
      .mockImplementation(() => {
        throw new Error('blocked')
      })
    const setItem = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new Error('blocked')
      })

    const mod = await freshModule()
    const { wrapEnabled, toggleWrap } = mod.usePlanetMapWrap()
    expect(wrapEnabled.value).toBe(true)
    expect(() => toggleWrap()).not.toThrow()
    expect(wrapEnabled.value).toBe(false)

    getItem.mockRestore()
    setItem.mockRestore()
  })
})
