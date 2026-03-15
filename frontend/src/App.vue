<script setup>
import { onMounted } from 'vue'
import { useWebSocket } from './composables/useWebSocket'
import { useLayoutStore } from './stores/layout'
import PanelWindow from './components/PanelWindow.vue'
import TimeControls from './components/TimeControls.vue'
import EventLog from './components/EventLog.vue'

const layout = useLayoutStore()
const { connect } = useWebSocket()

onMounted(() => {
  connect()
})

const panelComponents = {
  TimeControls,
  EventLog,
}
</script>

<template>
  <div class="relative w-full h-full">
    <!-- Title bar -->
    <div class="absolute top-0 left-0 right-0 h-8 bg-panel-header border-b border-panel-border flex items-center px-3 z-50">
      <span class="text-xs text-panel-muted tracking-widest uppercase">Outpost 3</span>
    </div>

    <!-- Panel workspace -->
    <div class="absolute top-8 left-0 right-0 bottom-0">
      <PanelWindow
        v-for="panel in layout.panels"
        :key="panel.id"
        :panel="panel"
        @focus="layout.focusPanel(panel.id)"
        @move="(pos) => layout.movePanel(panel.id, pos)"
        @resize="(size) => layout.resizePanel(panel.id, size)"
        @minimize="layout.toggleMinimize(panel.id)"
        @close="layout.closePanel(panel.id)"
      >
        <component :is="panelComponents[panel.component]" />
      </PanelWindow>

      <!-- Minimized panel bar -->
      <div class="absolute bottom-0 left-0 right-0 flex gap-1 p-1 bg-panel-header/50">
        <button
          v-for="panel in layout.minimizedPanels"
          :key="panel.id"
          class="px-2 py-1 text-xs bg-panel-bg border border-panel-border rounded hover:border-panel-accent transition-colors"
          @click="layout.toggleMinimize(panel.id)"
        >
          {{ panel.title }}
        </button>
      </div>
    </div>
  </div>
</template>
