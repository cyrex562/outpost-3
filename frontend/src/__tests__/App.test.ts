import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import App from '@/App.vue'

const routerBack = vi.fn()
let routePath = '/colony'

vi.mock('vue-router', () => ({
  useRouter: () => ({ back: routerBack, push: vi.fn() }),
  useRoute: () => ({ get path() { return routePath } }),
}))

vi.mock('@/composables/useGameSocket', () => ({
  useGameSocket: () => ({}),
}))

vi.mock('@/services/tauriBridge', () => ({
  isTauri: false,
  exitApp: vi.fn(),
  resetEngine: vi.fn(),
  saveGame: vi.fn(),
  listSaves: vi.fn(),
  loadGame: vi.fn(),
  setCustomDifficulty: vi.fn(),
  snapshot: vi.fn(),
}))

/** Push `history.length` above 1 so the App's Escape handler doesn't
 * short-circuit on the "no in-app history yet" guard for these tests. */
function primeHistory(): void {
  history.pushState({}, '', '#/colony')
}

describe('App.vue Escape-to-back handler (navigation rework #7 phase 1)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    routerBack.mockReset()
    routePath = '/colony'
    primeHistory()
  })

  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('navigates back on Escape when not on the root menu route', async () => {
    mount(App, { attachTo: document.body, global: { stubs: { RouterLink: true, RouterView: true } } })
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(routerBack).toHaveBeenCalledTimes(1)
  })

  it('does nothing on the root menu route', () => {
    routePath = '/'
    mount(App, { attachTo: document.body, global: { stubs: { RouterLink: true, RouterView: true } } })
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(routerBack).not.toHaveBeenCalled()
  })

  it('does not navigate away while focus is inside a text input', () => {
    mount(App, { attachTo: document.body, global: { stubs: { RouterLink: true, RouterView: true } } })
    const input = document.createElement('input')
    document.body.appendChild(input)
    input.focus()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))

    expect(routerBack).not.toHaveBeenCalled()
    input.remove()
  })
})
