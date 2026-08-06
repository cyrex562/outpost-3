<script setup lang="ts">
/**
 * Alert toast (events/alerts unification) — pops up whenever a new
 * alert-tier ('notable'/'urgent'/'blocking') entry lands in the unified
 * colony log (`worldStore.alertToast`, set by `handleServerMessage`),
 * regardless of which screen the player is currently on. Distinct from
 * `gameStore.toastMessage` (TurnControlBar's footer toast, which is for
 * command-result/error feedback, not gameplay alerts) — click to dismiss,
 * same interaction as that one, but positioned as a corner popup so it
 * doesn't compete with it.
 */
import { useWorldStore } from '@/stores/worldStore'

const worldStore = useWorldStore()
</script>

<template>
  <div
    v-if="worldStore.alertToast"
    class="alert-toast"
    :class="`tier-${worldStore.alertToast.tier}`"
    data-testid="alert-toast"
    :title="'Click to dismiss'"
    @click="worldStore.dismissAlertToast()"
  >
    <span class="alert-toast-tier">{{ worldStore.alertToast.tier }}</span>
    <span class="alert-toast-message">{{ worldStore.alertToast.message }}</span>
  </div>
</template>

<style scoped>
.alert-toast {
  position: fixed;
  top: 1rem;
  right: 1rem;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  max-width: min(24rem, calc(100vw - 2rem));
  padding: 0.5rem 0.75rem;
  border-radius: 4px;
  border: 1px solid var(--text-faint);
  background: var(--surface-2);
  box-shadow: 0 4px 16px var(--shadow-soft);
  cursor: pointer;
  font-size: 0.8rem;
}
.alert-toast-tier {
  text-transform: uppercase;
  font-size: 0.65rem;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}
.alert-toast-message { color: var(--accent-soft); overflow-wrap: anywhere; }

/* Matches AlertsPanel's tier color language. */
.alert-toast.tier-blocking { border-color: var(--danger-strong); }
.alert-toast.tier-blocking .alert-toast-tier { color: var(--danger-strong); }
.alert-toast.tier-urgent   { border-color: var(--danger-dim); }
.alert-toast.tier-urgent   .alert-toast-tier { color: var(--danger); }
.alert-toast.tier-notable  { border-color: var(--warn); }
.alert-toast.tier-notable  .alert-toast-tier { color: var(--warn-dim); }
</style>
