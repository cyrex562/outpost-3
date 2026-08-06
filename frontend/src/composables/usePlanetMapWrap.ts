/**
 * Shared "wrap the planet map east-west?" preference.
 *
 * `PlanetMap.width` wraps east-west (see `tauriBridge.ts`), and the hex map
 * renders enough shifted repeats of every layer to cover the viewBox so
 * panning west/east never runs into empty space. That repetition is useful
 * for following a route across the seam, but it also makes the map read as
 * an endless field rather than a finite globe, so it's a preference rather
 * than a fixed behavior.
 *
 * The ref lives at module scope, not per-component: every `PlanetHexMap` on
 * screen (and every one mounted later) reads the same value, so toggling it
 * on the surface preview and then opening the colony's planet view doesn't
 * show two different modes. Persisted to `localStorage` so the choice
 * survives a reload.
 */

import { ref, watch, type Ref } from 'vue'

const STORAGE_KEY = 'outpost3.planet-map.wrap'

/** Wrapping is on by default — it's the behavior the map shipped with. */
function loadWrapEnabled(): boolean {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    // Only an explicit "false" turns it off; a missing or corrupt entry
    // falls back to the default rather than guessing.
    if (raw === 'false') return false
    if (raw === 'true') return true
  } catch {
    // storage blocked (private mode, embedded webview) — non-fatal
  }
  return true
}

const wrapEnabled: Ref<boolean> = ref(loadWrapEnabled())

// `flush: 'sync'` so the write lands in the same tick as the toggle. The
// default pre-flush defers to a microtask, which is long enough for a
// reload triggered straight after a click to miss the new value.
watch(
  wrapEnabled,
  (on) => {
    try {
      window.localStorage.setItem(STORAGE_KEY, String(on))
    } catch {
      // storage blocked — the toggle still works for this session
    }
  },
  { flush: 'sync' },
)

export interface PlanetMapWrapControl {
  /** Whether the hex map repeats its layers east-west while panning. */
  wrapEnabled: Ref<boolean>
  /** Flip the preference (and persist it). */
  toggleWrap: () => void
}

/** Access the shared planet-map wrap preference. */
export function usePlanetMapWrap(): PlanetMapWrapControl {
  return {
    wrapEnabled,
    toggleWrap: () => {
      wrapEnabled.value = !wrapEnabled.value
    },
  }
}

/**
 * Reset the preference to its default. Test-only — component tests share the
 * module-scope ref, so one spec's toggle would otherwise leak into the next.
 */
export function __resetPlanetMapWrapForTests(): void {
  wrapEnabled.value = true
}

/**
 * Force the preference to a given value. Test-only — lets a spec start from
 * "the player had already turned wrapping off" without reaching into
 * localStorage and re-importing the module.
 */
export function __setPlanetMapWrapForTests(on: boolean): void {
  wrapEnabled.value = on
}
