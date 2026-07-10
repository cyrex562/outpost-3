<script setup lang="ts">
import { useGameSocket } from '@/composables/useGameSocket'
import { useWorldStore } from '@/stores/worldStore'

// Establish WebSocket connection for the lifetime of the app.
useGameSocket()

const store = useWorldStore()
</script>

<template>
  <div class="app">
    <header class="app-header">
      <h1>Outpost 3</h1>
      <span
        class="connection-status"
        :data-status="store.connectionStatus"
        data-testid="connection-status"
      >{{ store.connectionStatus }}</span>
      <div class="time-display" data-testid="time-display">
        Sol {{ store.sol }} · Month {{ store.month }}
      </div>
    </header>

    <main class="app-main">
      <RouterView />
    </main>
  </div>
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: monospace; background: #0a0a0f; color: #cdd; }

.app { display: flex; flex-direction: column; min-height: 100vh; }

.app-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.5rem 1rem;
  background: #111;
  border-bottom: 1px solid #333;
}

.app-header h1 { font-size: 1.1rem; color: #8cf; }

.connection-status[data-status="connected"] { color: #4c4; }
.connection-status[data-status="connecting"] { color: #cc4; }
.connection-status[data-status="disconnected"] { color: #888; }
.connection-status[data-status="error"] { color: #c44; }

.time-display { margin-left: auto; color: #8a8; font-size: 0.85rem; }

.app-main { flex: 1; padding: 1rem; }
</style>
