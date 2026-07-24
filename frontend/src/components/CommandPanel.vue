<script setup lang="ts">
/**
 * Command panel — issues turn-level commands for the colony currently in
 * view. The colony context comes from `gameStore.selectedColonyId` (kept in
 * sync with the `/colony/:colonyId` route param by `ColonyView`); switching
 * colonies happens on the planet map (map/nav plan phase A2), not from a
 * control here.
 *
 * The Found Colony / Set Directive / Research Tech controls were removed
 * (UI-rework): founding now originates from the system/surface map, and
 * research is driven from the tech tree — the old free-text tech-id and
 * placeholder-directive dialogs are obsolete.
 */

import { useGameStore } from '@/stores/game'

const gameStore = useGameStore()

async function advanceTurn(): Promise<void> {
  await gameStore.sendCommand({ kind: 'advance_sol' })
}
</script>

<template>
  <div class="command-panel" data-testid="command-panel">
    <h3 class="panel-title">Commands</h3>

    <div class="actions">
      <button
        class="btn btn-primary"
        :disabled="gameStore.busy"
        data-testid="btn-advance-turn"
        @click="advanceTurn"
      >
        Advance Turn
      </button>
    </div>

    <!-- Event toast -->
    <div
      v-if="gameStore.toastMessage"
      class="toast"
      data-testid="event-toast"
      @click="gameStore.dismissToast()"
    >
      {{ gameStore.toastMessage }}
    </div>
  </div>
</template>

<style scoped>
.command-panel {
  background: #111118;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 1rem;
}

.panel-title { color: #8cf; margin-bottom: 0.75rem; font-size: 0.9rem; letter-spacing: 0.05em; }

.actions { display: flex; flex-direction: column; gap: 0.5rem; }

.btn {
  background: #1a1a28;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  padding: 0.4rem 0.75rem;
  font-family: monospace;
  font-size: 0.82rem;
  cursor: pointer;
  text-align: left;
  transition: background 0.1s;
}

.btn:hover:not(:disabled) { background: #222236; }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-primary { border-color: #468; color: #8cf; }
.btn-primary:hover:not(:disabled) { background: #182030; }

.toast {
  margin-top: 0.75rem;
  background: #1a2020;
  border: 1px solid #2a5a3a;
  border-radius: 3px;
  color: #6c9;
  padding: 0.4rem 0.6rem;
  font-size: 0.78rem;
  cursor: pointer;
}
</style>
