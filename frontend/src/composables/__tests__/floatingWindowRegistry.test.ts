import { describe, expect, it } from 'vitest'
import { createFloatingWindowRegistry } from '@/composables/floatingWindowRegistry'

describe('floatingWindowRegistry snapMove', () => {
  it('snaps a window\'s left edge to a sibling\'s right edge within threshold', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 0, y: 0, w: 200, h: 100 })
    reg.register('b', { x: 500, y: 0, w: 200, h: 100 })

    // Dragging "b" so its left edge (500) lands 4px short of "a"'s right edge (200).
    const snapped = reg.snapMove('b', { x: 204, y: 0, w: 200, h: 100 }, null, null)
    expect(snapped.x).toBe(200)
  })

  it('snaps the trailing (right) edge just as well as the leading edge', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 500, y: 0, w: 200, h: 100 })
    reg.register('b', { x: 0, y: 0, w: 200, h: 100 })

    // "b"'s right edge (0+200=200) approaches "a"'s left edge (500) from the left... use a
    // closer scenario: b's right edge nearly touches a's left edge.
    const snapped = reg.snapMove('b', { x: 296, y: 0, w: 200, h: 100 }, null, null)
    // b's right edge would be 296+200=496, within 10 of a's left edge (500) -> offset +4
    expect(snapped.x).toBe(300)
  })

  it('does not snap when nothing is within the threshold', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 0, y: 0, w: 200, h: 100 })
    reg.register('b', { x: 500, y: 0, w: 200, h: 100 })

    const snapped = reg.snapMove('b', { x: 260, y: 0, w: 200, h: 100 }, null, null)
    expect(snapped.x).toBe(260)
  })

  it('snaps to host edges (x=0 and hostW) when provided', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 6, y: 4, w: 200, h: 100 })

    const snapped = reg.snapMove('a', { x: 4, y: 3, w: 200, h: 100 }, 1000, 800)
    expect(snapped.x).toBe(0)
    expect(snapped.y).toBe(0)
  })

  it('snaps to the host\'s right/bottom edge via the window\'s trailing edges', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 0, y: 0, w: 200, h: 100 })

    // Right edge (795+200=995) within 10px of hostW (1000).
    const snapped = reg.snapMove('a', { x: 795, y: 695, w: 200, h: 100 }, 1000, 800)
    expect(snapped.x).toBe(800) // 1000 - 200
    expect(snapped.y).toBe(700) // 800 - 100
  })

  it('ignores its own previously-registered rect when computing candidates', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 0, y: 0, w: 200, h: 100 })

    // Only "a" is registered; moving "a" itself must never snap against its own edges.
    const snapped = reg.snapMove('a', { x: 5, y: 5, w: 200, h: 100 }, null, null)
    expect(snapped).toEqual({ x: 5, y: 5 })
  })
})

describe('floatingWindowRegistry snapResize', () => {
  it('snaps the right edge to a sibling\'s left edge', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 500, y: 0, w: 200, h: 100 })
    reg.register('b', { x: 0, y: 0, w: 200, h: 100 })

    // Resizing "b" wider so its right edge (0+296=296) nears "a"'s left edge (500)... use a
    // closer number within threshold: right edge at 494 vs a's left edge 500.
    const snapped = reg.snapResize('b', { x: 0, y: 0, w: 494, h: 100 }, null, null)
    expect(snapped.w).toBe(500)
  })

  it('snaps the bottom edge to the host height', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 0, y: 0, w: 200, h: 100 })

    const snapped = reg.snapResize('a', { x: 0, y: 0, w: 200, h: 396 }, 1000, 400)
    expect(snapped.h).toBe(400)
  })

  it('does not snap when nothing is within threshold', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 500, y: 0, w: 200, h: 100 })
    reg.register('b', { x: 0, y: 0, w: 200, h: 100 })

    const snapped = reg.snapResize('b', { x: 0, y: 0, w: 260, h: 100 }, null, null)
    expect(snapped.w).toBe(260)
  })
})

describe('floatingWindowRegistry z-order', () => {
  it('bringToFront returns strictly increasing z-indices across calls', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 0, y: 0, w: 10, h: 10 })
    reg.register('b', { x: 0, y: 0, w: 10, h: 10 })

    const z1 = reg.bringToFront('a')
    const z2 = reg.bringToFront('b')
    const z3 = reg.bringToFront('a')
    expect(z2).toBeGreaterThan(z1)
    expect(z3).toBeGreaterThan(z2)
  })
})

describe('floatingWindowRegistry lifecycle', () => {
  it('unregister removes a window from future snap candidates', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 0, y: 0, w: 200, h: 100 })
    reg.register('b', { x: 500, y: 0, w: 200, h: 100 })
    reg.unregister('a')

    const snapped = reg.snapMove('b', { x: 204, y: 0, w: 200, h: 100 }, null, null)
    expect(snapped.x).toBe(204) // "a" is gone, nothing to snap to
  })

  it('update() reflects a moved sibling\'s latest position in later snap checks', () => {
    const reg = createFloatingWindowRegistry()
    reg.register('a', { x: 0, y: 0, w: 200, h: 100 })
    reg.register('b', { x: 900, y: 0, w: 200, h: 100 })

    reg.update('a', { x: 300, y: 0, w: 200, h: 100 })

    const snapped = reg.snapMove('b', { x: 504, y: 0, w: 200, h: 100 }, null, null)
    expect(snapped.x).toBe(500) // a's new right edge (300+200)
  })
})
