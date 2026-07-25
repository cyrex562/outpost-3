import { describe, expect, it, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import FloatingWindow from '@/components/FloatingWindow.vue'

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

  it('opens filling the host with fill-host, insetting by initialX', async () => {
    const restore = withHostSize(1600, 900)
    try {
      const wrapper = mountWindow({ fillHost: true })
      // The onMounted resize is reactive, so let Vue flush it to the DOM.
      await wrapper.vm.$nextTick()
      const s = styleOf(wrapper.get('[data-testid="floating-window"]').element)
      // Inset 10 (initialX) on every side — not the 400x300 initial size.
      expect(s.left).toBe('10px')
      expect(s.top).toBe('10px')
      expect(s.width).toBe('1580px')
      expect(s.height).toBe('880px')
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
