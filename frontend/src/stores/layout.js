import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

let nextZ = 10

function makePanel(id, title, component, x, y, w = 400, h = 300) {
  return {
    id,
    title,
    component,
    x,
    y,
    w,
    h,
    z: nextZ++,
    minimized: false,
    visible: true,
  }
}

export const useLayoutStore = defineStore('layout', () => {
  const allPanels = ref([
    makePanel('time-controls', 'Time Controls', 'TimeControls', 20, 20, 380, 160),
    makePanel('event-log', 'Event Log', 'EventLog', 20, 200, 500, 400),
    makePanel('ship-status', 'Ship Status', 'ShipStatus', 540, 20, 340, 580),
  ])

  const panels = computed(() =>
    allPanels.value.filter((p) => p.visible && !p.minimized)
  )

  const minimizedPanels = computed(() =>
    allPanels.value.filter((p) => p.visible && p.minimized)
  )

  function focusPanel(id) {
    const panel = allPanels.value.find((p) => p.id === id)
    if (panel) {
      panel.z = nextZ++
    }
  }

  function movePanel(id, { x, y }) {
    const panel = allPanels.value.find((p) => p.id === id)
    if (panel) {
      panel.x = x
      panel.y = y
    }
  }

  function resizePanel(id, { w, h }) {
    const panel = allPanels.value.find((p) => p.id === id)
    if (panel) {
      panel.w = Math.max(200, w)
      panel.h = Math.max(80, h)
    }
  }

  function toggleMinimize(id) {
    const panel = allPanels.value.find((p) => p.id === id)
    if (panel) {
      panel.minimized = !panel.minimized
    }
  }

  function closePanel(id) {
    const panel = allPanels.value.find((p) => p.id === id)
    if (panel) {
      panel.visible = false
    }
  }

  return {
    allPanels,
    panels,
    minimizedPanels,
    focusPanel,
    movePanel,
    resizePanel,
    toggleMinimize,
    closePanel,
  }
})
