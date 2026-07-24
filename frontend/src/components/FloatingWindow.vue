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
 */

import { onMounted, onUnmounted, reactive } from 'vue'

const props = defineProps<{
  title: string
  /** localStorage key for this window's persisted `{x,y,w,h}`. */
  storageKey: string
  initialX?: number
  initialY?: number
  initialWidth?: number
  initialHeight?: number
}>()

interface Rect {
  x: number
  y: number
  w: number
  h: number
}

const MIN_W = 240
const MIN_H = 180

function loadPersisted(): Rect | null {
  try {
    const raw = window.localStorage.getItem(props.storageKey)
    if (!raw) return null
    const p = JSON.parse(raw) as Rect
    if (
      typeof p.x === 'number' &&
      typeof p.y === 'number' &&
      typeof p.w === 'number' &&
      typeof p.h === 'number'
    ) {
      return { x: p.x, y: p.y, w: Math.max(MIN_W, p.w), h: Math.max(MIN_H, p.h) }
    }
  } catch {
    // corrupt entry — fall back to defaults
  }
  return null
}

const rect = reactive<Rect>(
  loadPersisted() ?? {
    x: props.initialX ?? 24,
    y: props.initialY ?? 24,
    w: props.initialWidth ?? 640,
    h: props.initialHeight ?? 460,
  },
)

function savePersisted(): void {
  try {
    window.localStorage.setItem(
      props.storageKey,
      JSON.stringify({ x: rect.x, y: rect.y, w: rect.w, h: rect.h }),
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
    rect.x = Math.max(0, originX + (e.clientX - originClientX))
    rect.y = Math.max(0, originY + (e.clientY - originClientY))
  } else if (resizing) {
    rect.w = Math.max(MIN_W, originW + (e.clientX - originClientX))
    rect.h = Math.max(MIN_H, originH + (e.clientY - originClientY))
  }
}

function onUp(): void {
  if (dragging || resizing) {
    dragging = false
    resizing = false
    savePersisted()
  }
}

onMounted(() => {
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
})
onUnmounted(() => {
  window.removeEventListener('mousemove', onMove)
  window.removeEventListener('mouseup', onUp)
})
</script>

<template>
  <div
    class="floating-window"
    data-testid="floating-window"
    :style="{ left: `${rect.x}px`, top: `${rect.y}px`, width: `${rect.w}px`, height: `${rect.h}px` }"
  >
    <div class="fw-titlebar" data-testid="fw-titlebar" @mousedown="onDragStart">
      <span class="fw-title">{{ title }}</span>
      <span class="fw-hint">drag to move · grip to resize</span>
    </div>
    <div class="fw-body">
      <slot />
    </div>
    <div class="fw-resize" data-testid="fw-resize" title="Resize" @mousedown="onResizeStart" />
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
