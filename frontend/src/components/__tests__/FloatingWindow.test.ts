import { describe, expect, it, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import FloatingWindow from '@/components/FloatingWindow.vue'
import {
  createFloatingWindowRegistry,
  FLOATING_WINDOW_REGISTRY_KEY,
  type FloatingWindowRegistry,
} from '@/composables/floatingWindowRegistry'

const KEY = 'test.floating-window'

function mountWindow(extraProps: Record<string, unknown> = {}) {
  return mount(FloatingWindow, {
    props: {
      title: 'Planet Map',
      storageKey: KEY,
      initialX: 10,
      initialY: 20,
      initialWidth: 400,
      initialHeight: 300,
      ...extraProps,
    },
    slots: { default: '<div data-testid="content">hi</div>' },
    attachTo: document.body,
  })
}

/**
 * jsdom reports every element's client box as 0, so host-relative sizing can't
 * be exercised without stubbing it. Patched on `Element.prototype` rather than
 * a specific node because Vue Test Utils' mount container is not addressable
 * from here — the component reads `rootRef.parentElement`, whatever that is.
 * Returns a restore function.
 */
function withHostSize(w: number, h: number): () => void {
  const cw = Object.getOwnPropertyDescriptor(Element.prototype, 'clientWidth')
  const ch = Object.getOwnPropertyDescriptor(Element.prototype, 'clientHeight')
  Object.defineProperty(Element.prototype, 'clientWidth', { configurable: true, get: () => w })
  Object.defineProperty(Element.prototype, 'clientHeight', { configurable: true, get: () => h })
  return () => {
    if (cw) Object.defineProperty(Element.prototype, 'clientWidth', cw)
    else delete (Element.prototype as unknown as Record<string, unknown>).clientWidth
    if (ch) Object.defineProperty(Element.prototype, 'clientHeight', ch)
    else delete (Element.prototype as unknown as Record<string, unknown>).clientHeight
  }
}

function styleOf(el: Element): CSSStyleDeclaration {
  return (el as HTMLElement).style
}

describe('FloatingWindow (UI-rework PR6)', () => {
  beforeEach(() => {
    window.localStorage.clear()
    document.body.innerHTML = ''
  })

  it('renders its slot content and the initial position/size', () => {
    const wrapper = mountWindow()
    expect(wrapper.find('[data-testid="content"]').exists()).toBe(true)
    const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
    expect(s.left).toBe('10px')
    expect(s.top).toBe('20px')
    expect(s.width).toBe('400px')
    expect(s.height).toBe('300px')
  })

  it('moves when the title bar is dragged, and persists the new position', async () => {
    const wrapper = mountWindow()
    await wrapper.get('[data-testid="fw-titlebar"]').trigger('mousedown', { button: 0, clientX: 100, clientY: 100 })
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 160, clientY: 145 }))
    window.dispatchEvent(new MouseEvent('mouseup'))
    await wrapper.vm.$nextTick()

    const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
    // moved by (+60, +45) from (10, 20)
    expect(s.left).toBe('70px')
    expect(s.top).toBe('65px')

    const saved = JSON.parse(window.localStorage.getItem(KEY)!)
    expect(saved).toMatchObject({ x: 70, y: 65 })
  })

  it('resizes when the corner grip is dragged, clamped to a minimum', async () => {
    const wrapper = mountWindow()
    await wrapper.get('[data-testid="fw-resize"]').trigger('mousedown', { button: 0, clientX: 400, clientY: 300 })
    // Drag far up-left to shrink well below the minimum.
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 50, clientY: 50 }))
    window.dispatchEvent(new MouseEvent('mouseup'))
    await wrapper.vm.$nextTick()

    const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
    // Clamped to MIN_W (240) / MIN_H (180), not the negative raw value.
    expect(s.width).toBe('240px')
    expect(s.height).toBe('180px')
  })

  it('does not drag the window above/left of its container (clamps to 0)', async () => {
    const wrapper = mountWindow()
    await wrapper.get('[data-testid="fw-titlebar"]').trigger('mousedown', { button: 0, clientX: 100, clientY: 100 })
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 0, clientY: 0 }))
    window.dispatchEvent(new MouseEvent('mouseup'))
    await wrapper.vm.$nextTick()

    const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
    expect(s.left).toBe('0px')
    expect(s.top).toBe('0px')
  })

  it('shows no close button by default, and emits close when closable is set', async () => {
    const bare = mountWindow()
    expect(bare.find('[data-testid="fw-close"]').exists()).toBe(false)

    const wrapper = mountWindow({ closable: true })
    expect(wrapper.emitted('close')).toBeUndefined()
    await wrapper.get('[data-testid="fw-close"]').trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('restores a persisted position on mount', () => {
    window.localStorage.setItem(KEY, JSON.stringify({ x: 200, y: 120, w: 500, h: 350 }))
    const wrapper = mountWindow()
    const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
    expect(s.left).toBe('200px')
    expect(s.top).toBe('120px')
    expect(s.width).toBe('500px')
    expect(s.height).toBe('350px')
  })
})

describe('FloatingWindow host-relative sizing (issue #320)', () => {
  beforeEach(() => {
    window.localStorage.clear()
    document.body.innerHTML = ''
  })

  it('opens filling the host with fill-host, per-axis inset', async () => {
    const restore = withHostSize(1600, 900)
    try {
      const wrapper = mountWindow({ fillHost: true })
      // The onMounted resize is reactive, so let Vue flush it to the DOM.
      await wrapper.vm.$nextTick()
      const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      // Inset by initialX (10) horizontally and initialY (20) vertically —
      // not the 400x300 initial size, and not initialX on all four sides.
      expect(s.left).toBe('10px')
      expect(s.top).toBe('20px')
      expect(s.width).toBe('1580px')
      expect(s.height).toBe('860px')
    } finally {
      restore()
    }
  })

  it('keeps the explicit initial size when the host cannot be measured', () => {
    // No host stub: jsdom reports 0, which must not collapse the window.
    const wrapper = mountWindow({ fillHost: true })
    const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
    expect(s.width).toBe('400px')
    expect(s.height).toBe('300px')
  })

  it('lets a persisted geometry win over fill-host', async () => {
    const restore = withHostSize(1600, 900)
    try {
      window.localStorage.setItem(KEY, JSON.stringify({ x: 40, y: 30, w: 620, h: 400 }))
      const wrapper = mountWindow({ fillHost: true })
      await wrapper.vm.$nextTick()
      const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      expect(s.width).toBe('620px')
      expect(s.height).toBe('400px')
    } finally {
      restore()
    }
  })

  it('clamps a geometry persisted on a larger display back inside the host', async () => {
    const restore = withHostSize(800, 600)
    try {
      window.localStorage.setItem(KEY, JSON.stringify({ x: 1200, y: 700, w: 1900, h: 1000 }))
      const wrapper = mountWindow()
      await wrapper.vm.$nextTick()
      const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      // Size capped to the host, then the origin pulled back in so the window
      // is actually reachable rather than parked off-screen.
      expect(s.width).toBe('800px')
      expect(s.height).toBe('600px')
      expect(s.left).toBe('0px')
      expect(s.top).toBe('0px')
    } finally {
      restore()
    }
  })

  it('maximises to the host flush and restores the previous geometry', async () => {
    const restore = withHostSize(1200, 800)
    try {
      const wrapper = mountWindow()
      const win = wrapper.get('[data-testid="floating-window"]')

      await wrapper.get('[data-testid="fw-maximise"]').trigger('click')
      let s = styleOf(win.element)
      expect(s.left).toBe('0px')
      expect(s.top).toBe('0px')
      expect(s.width).toBe('1200px')
      expect(s.height).toBe('800px')
      expect(win.classes()).toContain('maximised')
      // No corner grip while maximised — resizing is meaningless when flush.
      expect(wrapper.find('[data-testid="fw-resize"]').exists()).toBe(false)

      await wrapper.get('[data-testid="fw-maximise"]').trigger('click')
      s = styleOf(win.element)
      expect(s.width).toBe('400px')
      expect(s.height).toBe('300px')
      expect(win.classes()).not.toContain('maximised')
      expect(wrapper.find('[data-testid="fw-resize"]').exists()).toBe(true)
    } finally {
      restore()
    }
  })

  it('dragging a maximised window makes it free-floating again', async () => {
    const restore = withHostSize(1200, 800)
    try {
      const wrapper = mountWindow()
      const win = wrapper.get('[data-testid="floating-window"]')
      await wrapper.get('[data-testid="fw-maximise"]').trigger('click')
      expect(win.classes()).toContain('maximised')

      await wrapper.get('[data-testid="fw-titlebar"]').trigger('mousedown', { button: 0, clientX: 50, clientY: 50 })
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 80, clientY: 70 }))
      window.dispatchEvent(new MouseEvent('mouseup'))
      await wrapper.vm.$nextTick()

      expect(win.classes()).not.toContain('maximised')
      const s = styleOf(win.element)
      expect(s.left).toBe('30px')
      expect(s.top).toBe('20px')
    } finally {
      restore()
    }
  })

  it('re-fills the host when the app window resizes while maximised', async () => {
    let restore = withHostSize(1000, 700)
    try {
      const wrapper = mountWindow()
      await wrapper.get('[data-testid="fw-maximise"]').trigger('click')
      expect(styleOf(wrapper.get('[data-testid="floating-window"]').element).width).toBe('1000px')

      restore()
      restore = withHostSize(1400, 900)
      window.dispatchEvent(new Event('resize'))
      await wrapper.vm.$nextTick()

      const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      expect(s.width).toBe('1400px')
      expect(s.height).toBe('900px')
    } finally {
      restore()
    }
  })

  it('survives a reload as maximised rather than looking flush but unmaximised', async () => {
    const restore = withHostSize(1200, 800)
    try {
      const first = mountWindow()
      await first.get('[data-testid="fw-maximise"]').trigger('click')
      first.unmount()

      // Remount from the persisted entry: the flag rides along with the
      // geometry, so the window doesn't come back looking maximised while the
      // button still offers to maximise it.
      const wrapper = mountWindow()
      await wrapper.vm.$nextTick()
      const win = wrapper.get('[data-testid="floating-window"]')
      expect(win.classes()).toContain('maximised')
      expect(wrapper.get('[data-testid="fw-maximise"]').attributes('aria-pressed')).toBe('true')

      // ...and restore still returns to the pre-maximise geometry.
      await wrapper.get('[data-testid="fw-maximise"]').trigger('click')
      const s = styleOf(win.element)
      expect(s.width).toBe('400px')
      expect(s.height).toBe('300px')
    } finally {
      restore()
    }
  })

  it('keeps an untouched fill-host window filling a host that grows', async () => {
    let restore = withHostSize(1000, 700)
    try {
      const wrapper = mountWindow({ fillHost: true })
      await wrapper.vm.$nextTick()
      expect(styleOf(wrapper.get('[data-testid="floating-window"]').element).width).toBe('980px')

      restore()
      restore = withHostSize(1600, 900)
      window.dispatchEvent(new Event('resize'))
      await wrapper.vm.$nextTick()

      const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      expect(s.width).toBe('1580px')
      expect(s.height).toBe('860px')
    } finally {
      restore()
    }
  })

  it('stops auto-filling once the player has moved the window', async () => {
    let restore = withHostSize(1000, 700)
    try {
      const wrapper = mountWindow({ fillHost: true })
      await wrapper.vm.$nextTick()

      await wrapper.get('[data-testid="fw-titlebar"]').trigger('mousedown', { button: 0, clientX: 50, clientY: 50 })
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 90, clientY: 80 }))
      window.dispatchEvent(new MouseEvent('mouseup'))
      await wrapper.vm.$nextTick()
      const dragged = styleOf(wrapper.get('[data-testid="floating-window"]').element).width

      restore()
      restore = withHostSize(1600, 900)
      window.dispatchEvent(new Event('resize'))
      await wrapper.vm.$nextTick()

      // A growing host must not overwrite the size the player just chose.
      expect(styleOf(wrapper.get('[data-testid="floating-window"]').element).width).toBe(dragged)
    } finally {
      restore()
    }
  })

  it('insets vertically by initialY, not initialX', async () => {
    const restore = withHostSize(1000, 700)
    try {
      const wrapper = mountWindow({ fillHost: true, initialX: 10, initialY: 40 })
      await wrapper.vm.$nextTick()
      const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      expect(s.left).toBe('10px')
      expect(s.top).toBe('40px')
      expect(s.width).toBe('980px')
      expect(s.height).toBe('620px')
    } finally {
      restore()
    }
  })

  it('leaves a user-set size alone on resize, only pulling it back in bounds', async () => {
    let restore = withHostSize(1600, 900)
    try {
      const wrapper = mountWindow()
      await wrapper.vm.$nextTick()
      const s0 = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      expect(s0.width).toBe('400px')

      // Shrinking the host below the window's own size clamps it...
      restore()
      restore = withHostSize(300, 240)
      window.dispatchEvent(new Event('resize'))
      await wrapper.vm.$nextTick()
      let s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      expect(s.width).toBe('300px')
      expect(s.height).toBe('240px')

      // ...but growing it back never grows the window past what the user chose.
      restore()
      restore = withHostSize(1600, 900)
      window.dispatchEvent(new Event('resize'))
      await wrapper.vm.$nextTick()
      s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      expect(s.width).toBe('300px')
      expect(s.height).toBe('240px')
    } finally {
      restore()
    }
  })
})

describe('FloatingWindow multi-window snapping + z-order (colony details multi-window redesign)', () => {
  beforeEach(() => {
    window.localStorage.clear()
    document.body.innerHTML = ''
  })

  /**
   * Mounts a `FloatingWindow` with a shared `registry` injected the same way
   * a real providing ancestor would — `global.provide` on two separate
   * `mount()` calls sharing the same registry instance simulates two
   * sibling windows under one parent, without needing an actual wrapper
   * component (the registry is a plain framework-agnostic object, so this
   * is equivalent).
   */
  function mountWithRegistry(
    registry: FloatingWindowRegistry,
    extraProps: Record<string, unknown> = {},
  ) {
    return mount(FloatingWindow, {
      props: {
        title: 'Window',
        storageKey: `${KEY}.${extraProps.windowId ?? 'default'}`,
        initialX: 0,
        initialY: 0,
        initialWidth: 200,
        initialHeight: 100,
        ...extraProps,
      },
      global: { provide: { [FLOATING_WINDOW_REGISTRY_KEY as symbol]: registry } },
      slots: { default: '<div data-testid="content">hi</div>' },
      attachTo: document.body,
    })
  }

  it('snaps a dragged window to a sibling registered under the same registry', async () => {
    const registry = createFloatingWindowRegistry()
    // Sibling sits with its left edge at x=300.
    mountWithRegistry(registry, { windowId: 'a', initialX: 300, initialWidth: 200 })
    const b = mountWithRegistry(registry, { windowId: 'b', initialX: 0, initialWidth: 200 })

    // Drag "b" so its right edge (0+200=200 -> after drag ~296) lands just
    // short of "a"'s left edge (300).
    await b.get('[data-testid="fw-titlebar"]').trigger('mousedown', { button: 0, clientX: 0, clientY: 0 })
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 96, clientY: 0 }))
    window.dispatchEvent(new MouseEvent('mouseup'))
    await b.vm.$nextTick()

    const s = styleOf(b.get('[data-testid="floating-window"]').element)
    // b's right edge should land exactly on a's left edge (300), not the raw
    // dragged position (96) — i.e. x snapped to 300-200=100.
    expect(s.left).toBe('100px')
  })

  it('does not snap standalone windows with no registry provided', async () => {
    // No provide() at all — matches PlanetView.vue's existing single-window usage.
    const wrapper = mountWindow({ windowId: 'solo' })
    await wrapper.get('[data-testid="fw-titlebar"]').trigger('mousedown', { button: 0, clientX: 100, clientY: 100 })
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 160, clientY: 145 }))
    window.dispatchEvent(new MouseEvent('mouseup'))
    await wrapper.vm.$nextTick()

    const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
    expect(s.left).toBe('70px')
    expect(s.top).toBe('65px')
  })

  it('warns but still functions when windowId is given without a providing registry', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    try {
      const wrapper = mountWindow({ windowId: 'orphan' })
      expect(wrapper.find('[data-testid="floating-window"]').exists()).toBe(true)
      expect(warn).toHaveBeenCalledTimes(1)
      expect(warn.mock.calls[0][0]).toContain('orphan')
    } finally {
      warn.mockRestore()
    }
  })

  it('brings a window to the front of the shared z-order on click', async () => {
    const registry = createFloatingWindowRegistry()
    const a = mountWithRegistry(registry, { windowId: 'a' })
    const b = mountWithRegistry(registry, { windowId: 'b' })
    // onMounted's `bringToFront` call updates a ref; Vue batches the DOM
    // patch reflecting it, so it isn't visible in `style` until a tick passes.
    await a.vm.$nextTick()
    await b.vm.$nextTick()

    // "b" mounted after "a", so it's already on top by mount order — click "a"
    // to bring it to front instead.
    const zBeforeA = styleOf(a.get('[data-testid="floating-window"]').element).zIndex
    const zBeforeB = styleOf(b.get('[data-testid="floating-window"]').element).zIndex
    expect(Number(zBeforeB)).toBeGreaterThan(Number(zBeforeA))

    await a.get('.fw-body').trigger('mousedown')
    await a.vm.$nextTick()

    const zAfterA = styleOf(a.get('[data-testid="floating-window"]').element).zIndex
    expect(Number(zAfterA)).toBeGreaterThan(Number(zBeforeB))
  })

  it('brings a window to front even when the click is stopped from bubbling (the resize grip)', async () => {
    const registry = createFloatingWindowRegistry()
    const a = mountWithRegistry(registry, { windowId: 'a' })
    const b = mountWithRegistry(registry, { windowId: 'b' })
    await a.vm.$nextTick()
    await b.vm.$nextTick()
    const zBeforeB = styleOf(b.get('[data-testid="floating-window"]').element).zIndex

    // The resize grip's mousedown handler calls stopPropagation() (so it
    // doesn't also start a title-bar drag) — focus-on-click uses the
    // capture phase specifically so it still fires despite that.
    await a.get('[data-testid="fw-resize"]').trigger('mousedown', { button: 0, clientX: 0, clientY: 0 })
    window.dispatchEvent(new MouseEvent('mouseup'))
    await a.vm.$nextTick()

    const zAfterA = styleOf(a.get('[data-testid="floating-window"]').element).zIndex
    expect(Number(zAfterA)).toBeGreaterThan(Number(zBeforeB))
  })

  it('stops offering itself as a snap target after being unmounted', async () => {
    const registry = createFloatingWindowRegistry()
    const a = mountWithRegistry(registry, { windowId: 'a', initialX: 300, initialWidth: 200 })
    a.unmount()

    const b = mountWithRegistry(registry, { windowId: 'b', initialX: 0, initialWidth: 200 })
    await b.get('[data-testid="fw-titlebar"]').trigger('mousedown', { button: 0, clientX: 0, clientY: 0 })
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 96, clientY: 0 }))
    window.dispatchEvent(new MouseEvent('mouseup'))
    await b.vm.$nextTick()

    // "a" is gone — no snap should occur, so the raw dragged position stands.
    const s = styleOf(b.get('[data-testid="floating-window"]').element)
    expect(s.left).toBe('96px')
  })
})
