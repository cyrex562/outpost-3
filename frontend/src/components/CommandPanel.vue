<script setup lang="ts">
/**
 * Command panel — lets the player issue game commands for the colony
 * currently in view. The colony context comes from
 * `gameStore.selectedColonyId` (kept in sync with the `/colony/:colonyId`
 * route param by `ColonyView`); switching colonies happens on the planet
 * map (map/nav plan phase A2), not from a control in this panel.
 */

import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useGameStore } from '@/stores/game'

const gameStore = useGameStore()
const router = useRouter()

// Colony context comes from `gameStore.selectedColonyId`, which `ColonyView`
// keeps in sync with the `/colony/:colonyId` route param. Switching between
// colonies happens on the planet map (map/nav plan phase A2), which replaced
// this panel's old colony dropdown and `ColonyView`'s tab bar.

// ─── Set directive dialog ─────────────────────────────────────────────────────

const showDirectiveDialog = ref(false)
const directiveLabel = ref('')
const directivePriority = ref(50)

async function setDirective(): Promise<void> {
  const colId = gameStore.selectedColonyId
  if (!colId || !directiveLabel.value.trim()) return
  await gameStore.sendCommand({
    kind: 'set_directive',
    directive: {
      id: crypto.randomUUID(),
      colony_id: colId,
      label: directiveLabel.value.trim(),
      priority: directivePriority.value,
      condition: { kind: 'always' },
      action: { kind: 'noop' },
    },
  })
  directiveLabel.value = ''
  showDirectiveDialog.value = false
}

// ─── Research tech ────────────────────────────────────────────────────────────

const showResearchDialog = ref(false)
const techId = ref('')

async function researchTech(): Promise<void> {
  if (!techId.value.trim()) return
  await gameStore.sendCommand({ kind: 'research_tech', tech_id: techId.value.trim() })
  techId.value = ''
  showResearchDialog.value = false
}

// ─── Advance turn ─────────────────────────────────────────────────────────────

async function advanceTurn(): Promise<void> {
  await gameStore.sendCommand({ kind: 'advance_sol' })
}
</script>

<template>
  <div class="command-panel" data-testid="command-panel">
    <h3 class="panel-title">Commands</h3>

    <!-- Action buttons -->
    <div class="actions">
      <button
        class="btn btn-primary"
        :disabled="gameStore.busy"
        data-testid="btn-advance-turn"
        @click="advanceTurn"
      >
        Advance Turn
      </button>

      <button
        class="btn"
        data-testid="btn-found-colony"
        @click="router.push('/found')"
      >
        Found Colony
      </button>

      <button
        class="btn"
        :disabled="!gameStore.selectedColonyId"
        data-testid="btn-set-directive"
        @click="showDirectiveDialog = true"
      >
        Set Directive
      </button>

      <button
        class="btn"
        data-testid="btn-research-tech"
        @click="showResearchDialog = true"
      >
        Research Tech
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

    <!-- Set directive dialog -->
    <div v-if="showDirectiveDialog" class="dialog-backdrop">
      <div class="dialog" data-testid="set-directive-dialog">
        <h4>Set Directive</h4>
        <label class="field-label">Label</label>
        <input v-model="directiveLabel" class="input" placeholder="Directive label" />
        <label class="field-label">Priority (0–100)</label>
        <input v-model.number="directivePriority" class="input" type="number" min="0" max="100" />
        <div class="dialog-actions">
          <button class="btn btn-primary" @click="setDirective">Set</button>
          <button class="btn" @click="showDirectiveDialog = false">Cancel</button>
        </div>
      </div>
    </div>

    <!-- Research tech dialog -->
    <div v-if="showResearchDialog" class="dialog-backdrop">
      <div class="dialog" data-testid="research-tech-dialog">
        <h4>Research Tech</h4>
        <label class="field-label">Tech ID</label>
        <input v-model="techId" class="input" placeholder="e.g. advanced_mining" />
        <div class="dialog-actions">
          <button class="btn btn-primary" @click="researchTech">Research</button>
          <button class="btn" @click="showResearchDialog = false">Cancel</button>
        </div>
      </div>
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

.field { margin-bottom: 0.75rem; }
.field-label { display: block; font-size: 0.75rem; color: #778; margin-bottom: 0.25rem; }

.select, .input {
  width: 100%;
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.35rem 0.5rem;
  font-family: monospace;
  font-size: 0.85rem;
}

.select:focus, .input:focus {
  outline: none;
  border-color: #558;
}

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

.dialog-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.65);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.dialog {
  background: #15151f;
  border: 1px solid #446;
  border-radius: 6px;
  padding: 1.25rem;
  min-width: 280px;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.dialog h4 { color: #8cf; margin-bottom: 0.25rem; }

.dialog-actions { display: flex; gap: 0.5rem; margin-top: 0.5rem; }
</style>
