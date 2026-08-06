<script setup lang="ts">
/**
 * Balance tuning screen — a playtesting surface, not a player-facing one.
 *
 * Kept as its own route rather than a panel inside the colony screen: these
 * dials affect the whole simulation, not one colony, and mixing them into a
 * normal play screen invites changing them by accident.
 */
import BalanceTuningPanel from '@/components/BalanceTuningPanel.vue'
</script>

<template>
  <div class="balance-view" data-testid="balance-view">
    <div class="intro">
      <h2>Balance</h2>
      <p class="hint">
        Tune simulation scalars against the running game. Changes take effect on the next sol —
        advance time to see them. Values are multipliers; ×1.00 is unmodified.
      </p>
    </div>
    <BalanceTuningPanel />
  </div>
</template>

<style scoped>
.balance-view {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 46rem;
}
.intro h2 {
  color: var(--accent);
  margin: 0 0 0.25rem;
}
.hint {
  color: var(--text-muted);
  font-size: 0.85rem;
  margin: 0;
}
</style>
