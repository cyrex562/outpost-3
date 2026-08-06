/**
 * Light/dark theme selection.
 *
 * Three states, not two: `system` follows the OS's `prefers-color-scheme`
 * and is the default, while `light` and `dark` are explicit overrides. That
 * distinction matters — a player who picked "light" on a machine currently
 * set to dark should stay light, and a player who never touched the toggle
 * should follow their OS when it flips at sunset. Collapsing to a boolean
 * loses the difference between "wants light" and "wants whatever the OS
 * says, which happens to be light right now".
 *
 * The chosen mode is written to `<html data-theme>`, which `theme.css` reads
 * to pick a palette. In `system` mode the attribute is removed entirely so
 * the stylesheet's `prefers-color-scheme` media query takes over — that
 * keeps the OS-following path purely declarative, with no JS in the loop
 * once the attribute is cleared.
 *
 * State lives at module scope so every caller shares one value, and is
 * persisted so the choice survives a reload.
 */

import { computed, ref, watch, type ComputedRef, type Ref } from 'vue'

export type ThemeMode = 'system' | 'light' | 'dark'
export type ResolvedTheme = 'light' | 'dark'

const STORAGE_KEY = 'outpost3.theme'

/** Cycle order for the toggle: follow the OS, force light, force dark. */
const CYCLE: ThemeMode[] = ['system', 'light', 'dark']

function isThemeMode(v: string | null): v is ThemeMode {
  return v === 'system' || v === 'light' || v === 'dark'
}

function loadMode(): ThemeMode {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (isThemeMode(raw)) return raw
  } catch {
    // storage blocked (private mode, embedded webview) — non-fatal
  }
  return 'system'
}

const mode: Ref<ThemeMode> = ref(loadMode())

/** The OS preference, kept live so `system` mode tracks a mid-session flip. */
const systemPrefersDark = ref(prefersDark())

function prefersDark(): boolean {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
  } catch {
    // matchMedia is absent in some test environments — assume the app's
    // default look rather than throwing.
    return true
  }
}

try {
  const query = window.matchMedia('(prefers-color-scheme: dark)')
  const onChange = (e: MediaQueryListEvent) => {
    systemPrefersDark.value = e.matches
  }
  // `addEventListener` on a MediaQueryList is the modern API; older WebKit
  // (which the Tauri shell can still be built against) only has the
  // deprecated `addListener`.
  if (typeof query.addEventListener === 'function') {
    query.addEventListener('change', onChange)
  } else if (typeof (query as MediaQueryList).addListener === 'function') {
    ;(query as MediaQueryList).addListener(onChange)
  }
} catch {
  // no matchMedia — `system` mode just resolves to the default below
}

/** What the page actually renders as, after resolving `system`. */
const resolved: ComputedRef<ResolvedTheme> = computed(() => {
  if (mode.value === 'light') return 'light'
  if (mode.value === 'dark') return 'dark'
  return systemPrefersDark.value ? 'dark' : 'light'
})

/**
 * Reflect the mode onto `<html>`. `system` removes the attribute rather than
 * writing the resolved value, so the stylesheet's media query stays in
 * charge and the OS can flip the theme without JS running again.
 */
function applyMode(next: ThemeMode): void {
  const root = document.documentElement
  if (next === 'system') root.removeAttribute('data-theme')
  else root.setAttribute('data-theme', next)
}

// `flush: 'sync'` + `immediate` so the attribute is correct before the first
// paint, and so a reload straight after a click can't miss the write.
watch(
  mode,
  (next) => {
    applyMode(next)
    try {
      window.localStorage.setItem(STORAGE_KEY, next)
    } catch {
      // storage blocked — the choice still applies for this session
    }
  },
  { immediate: true, flush: 'sync' },
)

export interface ThemeControl {
  /** The player's choice: follow the OS, or force one of the two. */
  mode: Ref<ThemeMode>
  /** What that resolves to right now — always `light` or `dark`. */
  resolved: ComputedRef<ResolvedTheme>
  /** Step to the next mode in the cycle (system → light → dark → system). */
  cycleTheme: () => void
  /** Set a mode directly. */
  setTheme: (next: ThemeMode) => void
}

/** Access the shared theme selection. */
export function useTheme(): ThemeControl {
  return {
    mode,
    resolved,
    cycleTheme: () => {
      const i = CYCLE.indexOf(mode.value)
      mode.value = CYCLE[(i + 1) % CYCLE.length]
    },
    setTheme: (next: ThemeMode) => {
      mode.value = next
    },
  }
}

/** Reset to the default. Test-only — the module-scope ref is shared. */
export function __resetThemeForTests(): void {
  mode.value = 'system'
  systemPrefersDark.value = true
}

/** Force the OS preference. Test-only — `matchMedia` is not settable in jsdom. */
export function __setSystemPrefersDarkForTests(dark: boolean): void {
  systemPrefersDark.value = dark
}
