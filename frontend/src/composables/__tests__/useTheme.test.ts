import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  useTheme,
  __resetThemeForTests,
  __setSystemPrefersDarkForTests,
} from '@/composables/useTheme'

const STORAGE_KEY = 'outpost3.theme'

/** Re-import so the module-scope `loadMode()` runs against fresh storage. */
async function freshModule() {
  vi.resetModules()
  return import('@/composables/useTheme')
}

describe('useTheme', () => {
  beforeEach(() => {
    window.localStorage.clear()
    document.documentElement.removeAttribute('data-theme')
    __resetThemeForTests()
  })

  it('defaults to following the system preference', () => {
    expect(useTheme().mode.value).toBe('system')
  })

  it('leaves data-theme off in system mode so the CSS media query governs', () => {
    __resetThemeForTests()
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('resolves system mode from the OS preference, in both directions', () => {
    const { resolved } = useTheme()

    __setSystemPrefersDarkForTests(true)
    expect(resolved.value).toBe('dark')

    __setSystemPrefersDarkForTests(false)
    expect(resolved.value).toBe('light')
  })

  it('cycles system → light → dark → system', () => {
    const { mode, cycleTheme } = useTheme()

    expect(mode.value).toBe('system')
    cycleTheme()
    expect(mode.value).toBe('light')
    cycleTheme()
    expect(mode.value).toBe('dark')
    cycleTheme()
    expect(mode.value).toBe('system')
  })

  it('writes the explicit mode to <html data-theme>, and clears it for system', () => {
    const { setTheme } = useTheme()
    const root = document.documentElement

    setTheme('light')
    expect(root.getAttribute('data-theme')).toBe('light')

    setTheme('dark')
    expect(root.getAttribute('data-theme')).toBe('dark')

    setTheme('system')
    expect(root.hasAttribute('data-theme')).toBe(false)
  })

  it('lets an explicit choice override the OS preference in both directions', () => {
    const { setTheme, resolved } = useTheme()

    // OS says dark, player picked light — the pick wins.
    __setSystemPrefersDarkForTests(true)
    setTheme('light')
    expect(resolved.value).toBe('light')

    // ...and the reverse, which a boolean "isDark" toggle would get wrong.
    __setSystemPrefersDarkForTests(false)
    setTheme('dark')
    expect(resolved.value).toBe('dark')
  })

  it('persists the choice', () => {
    const { setTheme } = useTheme()
    setTheme('light')
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('light')
  })

  it('restores a persisted choice on reload', async () => {
    window.localStorage.setItem(STORAGE_KEY, 'dark')
    const mod = await freshModule()
    expect(mod.useTheme().mode.value).toBe('dark')
  })

  it('falls back to system when the stored value is not a known mode', async () => {
    window.localStorage.setItem(STORAGE_KEY, 'chartreuse')
    const mod = await freshModule()
    expect(mod.useTheme().mode.value).toBe('system')
  })

  it('still works when localStorage is unavailable', async () => {
    const getItem = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })

    const mod = await freshModule()
    const { mode, setTheme } = mod.useTheme()
    expect(mode.value).toBe('system')
    expect(() => setTheme('light')).not.toThrow()
    expect(mode.value).toBe('light')

    getItem.mockRestore()
    setItem.mockRestore()
  })

  it('shares one value across every caller', () => {
    const a = useTheme()
    const b = useTheme()
    a.setTheme('dark')
    expect(b.mode.value).toBe('dark')
  })
})
