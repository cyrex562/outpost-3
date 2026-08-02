<script setup lang="ts">
/**
 * Floating window (UI-rework PR6) — a self-contained draggable + resizable
 * container that floats (absolutely positioned) within its nearest
 * positioned ancestor. Drag the title bar to move; drag the bottom-right
 * grip to resize. Position + size persist to localStorage under `storageKey`,
 * so a window stays where the player left it across reloads.
 *
 * This generalizes the pan/resize/persist pattern already used by
 * SystemMapView, using global mouse listeners so a drag keeps tracking even
 * when the cursor leaves the window mid-gesture. The default slot is the
 * window's content, which fills the body and manages its own overflow.
 *
 * Sizing (issue #320): with `fill-host`, a window that has never been moved
 * opens filling its host rather than at a fixed pixel size, so a large display
 * isn't mostly empty. A maximise/restore toggle and a clamp-to-host pass on
 * app resize keep it usable afterwards — see `fitToHost`/`clampToHost`.
 *
 * Multi-window snapping + z-order (colony details multi-window redesign):
 * when a `windowId` prop is given AND an ancestor has `provide`d a
 * `FloatingWindowRegistry` (`floatingWindowRegistry.ts`), this window
 * registers its rect there, snaps its dragged/resized edges against every
 * other registered window's edges (and the host's edges) within a small
 * threshold, and bumps itself to the front of a shared z-order on any
 * click. Both `windowId` and the registry are optional — a `FloatingWindow`
 * used standalone (no id, no providing ancestor) behaves exactly as before.
 */

import { inject, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { FLOATING_WINDOW_REGISTRY_KEY } from '@/composables/floatingWindowRegistry'

const props = defineProps<{
  title: string
  /** localStorage key for this window's persisted `{x,y,w,h}`. */
  storageKey: string
  initialX?: number
  initialY?: number
  initialWidth?: number
  initialHeight?: number
  /**
   * Open filling the host — inset horizontally by `initialX` and vertically by
   * `initialY` — instead of at `initialWidth`×`initialHeight`. Such a window
   * keeps tracking the host until the player moves, resizes, or maximises it.
   * A persisted geometry still wins; this is only the untouched default.
   * Ignored when the host can't be measured (e.g. jsdom), where the explicit
   * initial size is used instead.
   */
  fillHost?: boolean
  /** Show a dismiss (×) button in the title bar that emits `close`. */
  closable?: boolean
  /** Opts into edge-snapping + shared z-order against sibling windows — see
   * the multi-window snapping doc comment above. Ignored (with a console
   * warning) if no ancestor provided a registry. */
  windowId?: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

interface Rect {
  x: number
  y: number
  w: number
  h: number
}

const MIN_W = 240
const MIN_H = 180

function isRect(v: unknown): v is Rect {
  const p = v as Rect | null
  return (
    !!p &&
    typeof p.x === 'number' &&
    typeof p.y === 'number' &&
    typeof p.w === 'number' &&
    typeof p.h === 'number'
  )
}

/**
 * Persisted shape. `maximised`/`restore` ride along with the geometry so a
 * reload can't leave a window rendered flush with its host while the button
 * still offers to maximise it.
 */
interface Persisted extends Rect {
  maximised?: boolean
  restore?: Rect | null
}

function loadPersisted(): Persisted | null {
  try {
    const raw = window.localStorage.getItem(props.storageKey)
    if (!raw) return null
    const p = JSON.parse(raw) as Persisted
    if (isRect(p)) {
      return {
        x: p.x,
        y: p.y,
        w: Math.max(MIN_W, p.w),
        h: Math.max(MIN_H, p.h),
        maximised: p.maximised === true,
        restore: isRect(p.restore) ? p.restore : null,
      }
    }
  } catch {
    // corrupt entry — fall back to defaults
  }
  return null
}

const persisted = loadPersisted()
const hadPersisted = persisted !== null

const rect = reactive<Rect>(
  persisted
    ? { x: persisted.x, y: persisted.y, w: persisted.w, h: persisted.h }
    : {
        x: props.initialX ?? 24,
        y: props.initialY ?? 24,
        w: props.initialWidth ?? 640,
        h: props.initialHeight ?? 460,
      },
)

const rootRef = ref<HTMLElement | null>(null)

// ── Multi-window snapping + z-order (opt-in, see the doc comment above) ────

const registry = inject(FLOATING_WINDOW_REGISTRY_KEY, null)

if (props.windowId && !registry) {
  // A windowId with no providing ancestor is almost certainly a wiring bug
  // (the parent forgot to `provide` a registry), not a legitimate standalone
  // usage — standalone usage omits windowId entirely. Warn, don't throw:
  // the window still works fine, just without snapping/shared z-order.
  console.warn(
    `FloatingWindow: windowId="${props.windowId}" was given but no FloatingWindowRegistry was provided by an ancestor — snapping and shared z-order are disabled for this window.`,
  )
}

const zIndex = ref<number | null>(null)

/** Bump this window to the front of the shared z-order. No-op without a registry. */
function focusWindow(): void {
  if (registry && props.windowId) {
    zIndex.value = registry.bringToFront(props.windowId)
  }
}

/** Whether the window is currently maximised to its host. */
const maximised = ref(persisted?.maximised ?? false)
/** Geometry to return to when un-maximising. */
let restoreRect: Rect | null = persisted?.restore ?? null

/**
 * Whether the current geometry is the player's own choice rather than a
 * `fill-host` default. Only gestures persist geometry, so anything loaded from
 * storage was chosen deliberately. An untouched `fill-host` window keeps
 * tracking its host as the app resizes; once the player moves, resizes, or
 * maximises it, we stop second-guessing their size.
 */
const userSized = ref(hadPersisted)

/**
 * Inner size of the positioned ancestor this window floats within, or `null`
 * when it can't be measured — jsdom reports 0, and a 0-sized host would
 * otherwise collapse the window to its minimum.
 *
 * The host is the window's direct parent by convention; both call sites make
 * `FloatingWindow` the only child of a `position: relative` `.map-host`.
 * Wrapping it in another element would silently measure the wrapper instead.
 */
function hostSize(): { w: number; h: number } | null {
  const host = rootRef.value?.parentElement
  if (!host) return null
  const w = host.clientWidth
  const h = host.clientHeight
  if (w <= 0 || h <= 0) return null
  return { w, h }
}

/** Resize to fill the host, inset by `insetX`/`insetY` on the relevant sides. */
function fitToHost(insetX: number, insetY: number = insetX): boolean {
  const host = hostSize()
  if (!host) return false
  rect.x = insetX
  rect.y = insetY
  rect.w = Math.max(MIN_W, host.w - insetX * 2)
  rect.h = Math.max(MIN_H, host.h - insetY * 2)
  return true
}

/** The `fill-host` inset this window opens with. */
function openInset(): { x: number; y: number } {
  return { x: props.initialX ?? 0, y: props.initialY ?? 0 }
}

/**
 * Pull a window back inside the host. Guards against a geometry persisted on a
 * large display leaving the window mostly off-screen on a smaller one — the
 * size is only ever reduced, never grown, so a deliberate user size survives.
 */
function clampToHost(): void {
  const host = hostSize()
  if (!host) return
  rect.w = Math.max(MIN_W, Math.min(rect.w, host.w))
  rect.h = Math.max(MIN_H, Math.min(rect.h, host.h))
  rect.x = Math.max(0, Math.min(rect.x, host.w - rect.w))
  rect.y = Math.max(0, Math.min(rect.y, host.h - rect.h))
}

function toggleMaximise(): void {
  if (maximised.value) {
    if (restoreRect) Object.assign(rect, restoreRect)
    maximised.value = false
    clampToHost()
  } else {
    restoreRect = { x: rect.x, y: rect.y, w: rect.w, h: rect.h }
    // Flush against the host edges — a maximise should leave no margin.
    if (!fitToHost(0)) return
    maximised.value = true
  }
  userSized.value = true
  savePersisted()
}

/**
 * Keep the window sensible when the host changes size. A maximised window
 * re-fills; an untouched `fill-host` window keeps filling, so growing the app
 * doesn't leave it stranded at the old size; anything the player sized is only
 * pulled back inside the host.
 */
function onHostResize(): void {
  if (maximised.value) {
    fitToHost(0)
    savePersisted()
  } else if (!userSized.value && props.fillHost) {
    const inset = openInset()
    fitToHost(inset.x, inset.y)
  } else {
    clampToHost()
  }
}

function savePersisted(): void {
  try {
    window.localStorage.setItem(
      props.storageKey,
      JSON.stringify({
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
        maximised: maximised.value,
        restore: restoreRect,
      }),
    )
  } catch {
    // storage full or blocked — non-fatal
  }
}

// ── Drag (title bar) + resize (corner grip) ────────────────────────────────
// A single pointer origin serves both gestures; only one is active at a time.
let dragging = false
let resizing = false
let originClientX = 0
let originClientY = 0
let originX = 0
let originY = 0
let originW = 0
let originH = 0

function onDragStart(e: MouseEvent): void {
  if (e.button !== 0) return
  // Dragging a maximised window makes it free-floating again at its current
  // size — it is no longer flush with the host, so the flag would be a lie.
  maximised.value = false
  dragging = true
  originClientX = e.clientX
  originClientY = e.clientY
  originX = rect.x
  originY = rect.y
  e.preventDefault()
}

function onResizeStart(e: MouseEvent): void {
  if (e.button !== 0) return
  resizing = true
  originClientX = e.clientX
  originClientY = e.clientY
  originW = rect.w
  originH = rect.h
  e.preventDefault()
  // Don't let the grip's mousedown also start a title-bar drag.
  e.stopPropagation()
}

function onMove(e: MouseEvent): void {
  if (dragging) {
    // Clamp to the top-left so the title bar can't be dragged out of reach.
    let x = Math.max(0, originX + (e.clientX - originClientX))
    let y = Math.max(0, originY + (e.clientY - originClientY))
    if (registry && props.windowId) {
      const host = hostSize()
      const snapped = registry.snapMove(
        props.windowId,
        { x, y, w: rect.w, h: rect.h },
        host?.w ?? null,
        host?.h ?? null,
      )
      x = snapped.x
      y = snapped.y
    }
    rect.x = x
    rect.y = y
  } else if (resizing) {
    let w = Math.max(MIN_W, originW + (e.clientX - originClientX))
    let h = Math.max(MIN_H, originH + (e.clientY - originClientY))
    if (registry && props.windowId) {
      const host = hostSize()
      const snapped = registry.snapResize(
        props.windowId,
        { x: rect.x, y: rect.y, w, h },
        host?.w ?? null,
        host?.h ?? null,
      )
      w = snapped.w
      h = snapped.h
    }
    rect.w = w
    rect.h = h
  }
}

function onUp(): void {
  if (dragging || resizing) {
    dragging = false
    resizing = false
    // The geometry is now the player's choice, so stop auto-filling the host.
    userSized.value = true
    savePersisted()
  }
}

/**
 * Watches the host directly, so a layout change that doesn't resize the app
 * window (a splitter drag, a collapsing sidebar) is still picked up. `resize`
 * alone would miss those. Absent in jsdom, hence the guard.
 */
let hostObserver: ResizeObserver | null = null

onMounted(() => {
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
  window.addEventListener('resize', onHostResize)

  // A maximised window re-fills in case the host changed while it was gone;
  // otherwise a first run with `fill-host` opens filling the host. A persisted
  // geometry is the player's own choice and always wins — but is still pulled
  // in-bounds, so a size chosen on a bigger display can't overhang this one.
  const inset = openInset()
  if (maximised.value) {
    fitToHost(0)
  } else if (!hadPersisted && props.fillHost) {
    fitToHost(inset.x, inset.y)
  } else {
    clampToHost()
  }

  const host = rootRef.value?.parentElement
  if (host && typeof ResizeObserver !== 'undefined') {
    hostObserver = new ResizeObserver(onHostResize)
    hostObserver.observe(host)
  }

  if (registry && props.windowId) {
    registry.register(props.windowId, { ...rect })
    zIndex.value = registry.bringToFront(props.windowId)
  }
})
onUnmounted(() => {
  window.removeEventListener('mousemove', onMove)
  window.removeEventListener('mouseup', onUp)
  window.removeEventListener('resize', onHostResize)
  hostObserver?.disconnect()
  hostObserver = null
  if (registry && props.windowId) {
    registry.unregister(props.windowId)
  }
})

// Keep the registry's copy of this window's rect current — including
// changes from the host-resize/maximise paths above, not just drag/resize
// gestures — so a sibling window dragged later snaps against where this
// one actually is right now, not where it was at mount.
if (registry) {
  watch(
    () => ({ ...rect }),
    (r) => {
      if (props.windowId) registry.update(props.windowId, r)
    },
  )
}
</script>

<template>
  <div
    ref="rootRef"
    class="floating-window"
    :class="{ maximised }"
    :data-window-id="windowId"
    data-testid="floating-window"
    :style="{
      left: `${rect.x}px`,
      top: `${rect.y}px`,
      width: `${rect.w}px`,
      height: `${rect.h}px`,
      zIndex: zIndex ?? undefined,
    }"
    @mousedown.capture="focusWindow"
  >
    <div class="fw-titlebar" data-testid="fw-titlebar" @mousedown="onDragStart">
      <span class="fw-title">{{ title }}</span>
      <span class="fw-hint">{{ maximised ? 'maximised · drag to restore' : 'drag to move · grip to resize' }}</span>
      <!-- mousedown is stopped so hitting the button can't also start a drag. -->
      <button
        class="fw-max"
        type="button"
        data-testid="fw-maximise"
        :title="maximised ? 'Restore' : 'Maximise'"
        :aria-label="maximised ? 'Restore' : 'Maximise'"
        :aria-pressed="maximised"
        @mousedown.stop
        @click="toggleMaximise"
      >
        {{ maximised ? '❐' : '▢' }}
      </button>
      <!-- mousedown is stopped so hitting the button can't also start a drag. -->
      <button
        v-if="closable"
        class="fw-close"
        type="button"
        data-testid="fw-close"
        title="Close"
        aria-label="Close"
        @mousedown.stop
        @click="emit('close')"
      >
        ×
      </button>
    </div>
    <div class="fw-body">
      <slot />
    </div>
    <div
      v-if="!maximised"
      class="fw-resize"
      data-testid="fw-resize"
      title="Resize"
      @mousedown="onResizeStart"
    />
  </div>
</template>

<style scoped>
.floating-window {
  position: absolute;
  display: flex;
  flex-direction: column;
  background: #0b0b12;
  border: 1px solid #345;
  border-radius: 6px;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.5);
  overflow: hidden;
  z-index: 5;
}

.fw-titlebar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.35rem 0.6rem;
  background: #14141f;
  border-bottom: 1px solid #223;
  cursor: move;
  user-select: none;
}
.fw-title { color: #8cf; font-size: 0.82rem; font-weight: 600; }
.fw-hint { color: #556; font-size: 0.68rem; font-style: italic; margin-left: auto; }

/* A maximised window is flush with the host, so the rounded corners and drop
   shadow would just read as a rendering artefact against the host edge. */
.floating-window.maximised { border-radius: 0; box-shadow: none; }

.fw-max {
  background: #1a1a28;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  cursor: pointer;
  font-size: 0.72rem;
  line-height: 1;
  padding: 0.15rem 0.35rem;
}
.fw-max:hover { background: #22223a; color: #8cf; }

.fw-close {
  background: none;
  border: none;
  color: #889;
  cursor: pointer;
  font-size: 1.05rem;
  line-height: 1;
  padding: 0 0.2rem;
}
.fw-close:hover { color: #f89; }

.fw-body { flex: 1; min-height: 0; position: relative; overflow: hidden; }

.fw-resize {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 16px;
  height: 16px;
  cursor: nwse-resize;
  background: linear-gradient(135deg, transparent 50%, #446 50%, #446 60%, transparent 60%, transparent 70%, #446 70%, #446 80%, transparent 80%);
}
</style>
