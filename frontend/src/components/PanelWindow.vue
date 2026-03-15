<script setup>
import { ref } from 'vue'

const props = defineProps({
  panel: { type: Object, required: true },
})

const emit = defineEmits(['focus', 'move', 'resize', 'minimize', 'close'])

// ── Drag ──────────────────────────────────────────────
const dragging = ref(false)
let dragOffset = { x: 0, y: 0 }

function onDragStart(e) {
  if (e.target.closest('[data-no-drag]')) return
  dragging.value = true
  dragOffset = {
    x: e.clientX - props.panel.x,
    y: e.clientY - props.panel.y,
  }
  emit('focus')
  document.addEventListener('mousemove', onDragMove)
  document.addEventListener('mouseup', onDragEnd)
}

function onDragMove(e) {
  if (!dragging.value) return
  emit('move', {
    x: Math.max(0, e.clientX - dragOffset.x),
    y: Math.max(0, e.clientY - dragOffset.y),
  })
}

function onDragEnd() {
  dragging.value = false
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup', onDragEnd)
}

// ── Resize ────────────────────────────────────────────
const resizing = ref(false)
let resizeStart = { x: 0, y: 0, w: 0, h: 0 }

function onResizeStart(e) {
  resizing.value = true
  resizeStart = {
    x: e.clientX,
    y: e.clientY,
    w: props.panel.w,
    h: props.panel.h,
  }
  emit('focus')
  document.addEventListener('mousemove', onResizeMove)
  document.addEventListener('mouseup', onResizeEnd)
  e.stopPropagation()
}

function onResizeMove(e) {
  if (!resizing.value) return
  emit('resize', {
    w: resizeStart.w + (e.clientX - resizeStart.x),
    h: resizeStart.h + (e.clientY - resizeStart.y),
  })
}

function onResizeEnd() {
  resizing.value = false
  document.removeEventListener('mousemove', onResizeMove)
  document.removeEventListener('mouseup', onResizeEnd)
}
</script>

<template>
  <div
    class="absolute border border-panel-border rounded-sm shadow-lg bg-panel-bg flex flex-col overflow-hidden select-none"
    :style="{
      left: panel.x + 'px',
      top: panel.y + 'px',
      width: panel.w + 'px',
      height: panel.h + 'px',
      zIndex: panel.z,
    }"
    @mousedown="emit('focus')"
  >
    <!-- Header bar -->
    <div
      class="flex items-center justify-between px-2 h-7 bg-panel-header border-b border-panel-border cursor-move shrink-0"
      @mousedown="onDragStart"
    >
      <span class="text-xs text-panel-muted tracking-wide uppercase truncate">
        {{ panel.title }}
      </span>
      <div class="flex gap-1" data-no-drag>
        <button
          class="w-5 h-5 flex items-center justify-center text-panel-muted hover:text-panel-text text-xs rounded hover:bg-panel-border/50 transition-colors"
          @click="emit('minimize')"
          title="Minimize"
        >
          _
        </button>
        <button
          class="w-5 h-5 flex items-center justify-center text-panel-muted hover:text-panel-accent text-xs rounded hover:bg-panel-border/50 transition-colors"
          @click="emit('close')"
          title="Close"
        >
          ×
        </button>
      </div>
    </div>

    <!-- Content slot -->
    <div class="flex-1 overflow-auto p-2">
      <slot />
    </div>

    <!-- Resize handle -->
    <div
      class="absolute bottom-0 right-0 w-3 h-3 cursor-se-resize"
      @mousedown="onResizeStart"
    >
      <svg class="w-3 h-3 text-panel-border" viewBox="0 0 12 12">
        <path d="M10 12L12 10M6 12L12 6M2 12L12 2" stroke="currentColor" stroke-width="1" />
      </svg>
    </div>
  </div>
</template>
