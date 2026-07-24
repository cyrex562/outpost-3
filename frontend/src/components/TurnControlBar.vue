<script setup lang="ts">
/**
 * Turn control bar (UI-rework PR3) — the persistent footer. Shows the
 * current-turn indicator (Sol / Month) on the left and the Advance Turn
 * button pinned to the bottom-right of the screen, so advancing the turn is
 * reachable from every in-game screen (it used to live only in the colony
 * page's CommandPanel). Also hosts the global event toast, which likewise
 * moved out of CommandPanel.
 */

import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'

const world = useWorldStore()
const gameStore = useGameStore()

async function advanceTurn(): Promise<void> {
  await gameStore.sendCommand({ kind: 'advance_sol' })
}
</script>

<template>
  <footer class="turn-control-bar" data-testid="turn-control-bar">
    <div class="turn-indicator" data-testid="turn-indicator">
      Sol {{ world.sol }} · Month {{ world.month }}
    </div>

    <div
      v-if="gameStore.toastMessage"
      class="toast"
      data-testid="event-toast"
      @click="gameStore.dismissToast()"
    >
      {{ gameStore.toastMessage }}
    </div>

    <button
      class="btn-advance"
      :disabled="gameStore.busy"
      data-testid="btn-advance-turn"
      @click="advanceTurn"
    >
      Advance Turn ▶
    </button>
  </footer>
</template>

<style scoped>
.turn-control-bar {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.4rem 1rem;
  background: #111118;
  border-top: 1px solid #334;
}

.turn-indicator { color: #8a8; font-size: 0.85rem; white-space: nowrap; }

.toast {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: #1a2020;
  border: 1px solid #2a5a3a;
  border-radius: 3px;
  color: #6c9;
  padding: 0.3rem 0.6rem;
  font-size: 0.78rem;
  cursor: pointer;
}

/* Pin the advance button to the bottom-right of the screen. */
.btn-advance {
  margin-left: auto;
  background: #14202e;
  border: 1px solid #468;
  border-radius: 3px;
  color: #8cf;
  padding: 0.45rem 1.1rem;
  font-family: monospace;
  font-size: 0.9rem;
  font-weight: bold;
  cursor: pointer;
  white-space: nowrap;
}
.btn-advance:hover:not(:disabled) { background: #1b2c40; }
.btn-advance:disabled { opacity: 0.45; cursor: not-allowed; }
</style>
