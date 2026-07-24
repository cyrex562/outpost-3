import { describe, expect, it, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import FloatingWindow from '@/components/FloatingWindow.vue'

const KEY = 'test.floating-window'

function mountWindow() {
  return mount(FloatingWindow, {
    props: { title: 'Planet Map', storageKey: KEY, initialX: 10, initialY: 20, initialWidth: 400, initialHeight: 300 },
    slots: { default: '<div data-testid="content">hi</div>' },
    attachTo: document.body,
  })
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
