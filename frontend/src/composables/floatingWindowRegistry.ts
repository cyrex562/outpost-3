/**
 * Shared registry for a group of `FloatingWindow` instances that should
 * snap against each other's edges and maintain a shared click-to-front
 * z-order — e.g. the colony screen's panel windows (issue: colony details
 * multi-window redesign).
 *
 * A parent component creates one via `createFloatingWindowRegistry()` and
 * `provide`s it under `FLOATING_WINDOW_REGISTRY_KEY`; each `FloatingWindow`
 * that should participate passes a `windowId` prop and `inject`s it itself.
 * `inject` resolves to `undefined` for any `FloatingWindow` mounted without
 * a providing ancestor (e.g. `PlanetView.vue`'s single building-details
 * window), so snapping/z-order are purely opt-in — nothing here changes
 * behavior for a `FloatingWindow` used standalone.
 */

import { reactive, type InjectionKey } from 'vue'

export interface WindowRect {
  x: number
  y: number
  w: number
  h: number
}

/** Which of a rect's four edges a resize gesture actually moves — the other
 * two stay anchored. A single-side handle frees one; a corner handle frees
 * two (e.g. the north-west corner frees `top` and `left`). */
export interface FreeEdges {
  left: boolean
  right: boolean
  top: boolean
  bottom: boolean
}

export interface FloatingWindowRegistry {
  register(id: string, rect: WindowRect): void
  update(id: string, rect: WindowRect): void
  unregister(id: string): void
  /** Snapped `{x, y}` for a move gesture (size held fixed), or the input's
   * own x/y unchanged if nothing is within snapping distance. */
  snapMove(id: string, proposed: WindowRect, hostW: number | null, hostH: number | null): { x: number; y: number }
  /** Snapped rect for a resize gesture — only the edges marked `true` in
   * `free` move; the rest stay exactly as given in `proposed`. */
  snapResize(
    id: string,
    proposed: WindowRect,
    free: FreeEdges,
    hostW: number | null,
    hostH: number | null,
  ): WindowRect
  /** Moves `id` to the top of the z-order and returns its new z-index. */
  bringToFront(id: string): number
}

export const FLOATING_WINDOW_REGISTRY_KEY: InjectionKey<FloatingWindowRegistry> =
  Symbol('floatingWindowRegistry')

/** Pixel distance within which an edge snaps to a candidate. */
const SNAP_PX = 10

/** Below every dockview-era z-index this app used (in the 900s for its own
 * float-a-tab feature) — kept low so unrelated overlays (menus, toasts)
 * that assume a low baseline aren't buried under a focused colony window. */
const BASE_Z = 10

/**
 * Of the edges in `edges`, find whichever is closest to any value in
 * `candidates` (within `threshold`), and return the offset that would move
 * that edge exactly onto its candidate — or `null` if nothing is close
 * enough. A single offset is correct for every edge in `edges` because
 * they're assumed to move together (a pure translation for a move gesture,
 * or the offset is applied to the one moving edge for a resize gesture).
 */
function bestSnapOffset(edges: number[], candidates: number[], threshold: number): number | null {
  let best: number | null = null
  let bestDelta = threshold
  for (const edge of edges) {
    for (const candidate of candidates) {
      const delta = Math.abs(edge - candidate)
      if (delta <= bestDelta) {
        bestDelta = delta
        best = candidate - edge
      }
    }
  }
  return best
}

export function createFloatingWindowRegistry(): FloatingWindowRegistry {
  const rects = reactive(new Map<string, WindowRect>())
  let nextZ = BASE_Z

  function register(id: string, rect: WindowRect): void {
    rects.set(id, { ...rect })
  }

  function update(id: string, rect: WindowRect): void {
    rects.set(id, { ...rect })
  }

  function unregister(id: string): void {
    rects.delete(id)
  }

  function otherRects(id: string): WindowRect[] {
    const out: WindowRect[] = []
    for (const [otherId, rect] of rects) {
      if (otherId !== id) out.push(rect)
    }
    return out
  }

  function snapMove(
    id: string,
    proposed: WindowRect,
    hostW: number | null,
    hostH: number | null,
  ): { x: number; y: number } {
    const { w, h } = proposed
    let { x, y } = proposed

    const candidatesX: number[] = []
    const candidatesY: number[] = []
    for (const r of otherRects(id)) {
      candidatesX.push(r.x, r.x + r.w)
      candidatesY.push(r.y, r.y + r.h)
    }
    if (hostW !== null) candidatesX.push(0, hostW)
    if (hostH !== null) candidatesY.push(0, hostH)

    const dx = bestSnapOffset([x, x + w], candidatesX, SNAP_PX)
    if (dx !== null) x += dx
    const dy = bestSnapOffset([y, y + h], candidatesY, SNAP_PX)
    if (dy !== null) y += dy

    return { x, y }
  }

  function snapResize(
    id: string,
    proposed: WindowRect,
    free: FreeEdges,
    hostW: number | null,
    hostH: number | null,
  ): WindowRect {
    let { x, y, w, h } = proposed

    const candidatesX: number[] = []
    const candidatesY: number[] = []
    for (const r of otherRects(id)) {
      candidatesX.push(r.x, r.x + r.w)
      candidatesY.push(r.y, r.y + r.h)
    }
    if (hostW !== null) candidatesX.push(0, hostW)
    if (hostH !== null) candidatesY.push(0, hostH)

    // Each edge snaps independently against its own axis of candidates.
    // Left/top additionally shift x/y since they move the anchor itself
    // (right/bottom stay fixed by construction — only w/h track a left/top
    // move); right/bottom only ever change w/h, x/y untouched.
    if (free.right) {
      const d = bestSnapOffset([x + w], candidatesX, SNAP_PX)
      if (d !== null) w += d
    }
    if (free.left) {
      const d = bestSnapOffset([x], candidatesX, SNAP_PX)
      if (d !== null) {
        x += d
        w -= d
      }
    }
    if (free.bottom) {
      const d = bestSnapOffset([y + h], candidatesY, SNAP_PX)
      if (d !== null) h += d
    }
    if (free.top) {
      const d = bestSnapOffset([y], candidatesY, SNAP_PX)
      if (d !== null) {
        y += d
        h -= d
      }
    }

    return { x, y, w, h }
  }

  function bringToFront(_id: string): number {
    return nextZ++
  }

  return { register, update, unregister, snapMove, snapResize, bringToFront }
}
