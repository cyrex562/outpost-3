<script setup lang="ts">
/**
 * System stats bar (UI-rework PR3) — a full-width strip under the header
 * showing system-wide aggregates that aren't tied to a single colony:
 * cumulative research, colony count, total population, and the current
 * alert count. Shown on every in-game screen so these numbers are always
 * visible without opening the colony dashboard (where "System research"
 * used to live as a small line).
 */

import { computed } from 'vue'
import { useWorldStore } from '@/stores/worldStore'

const world = useWorldStore()

const colonyCount = computed(() => world.colonies.length)
const totalPopulation = computed(() =>
  world.colonies.reduce((sum, c) => sum + (c.population ?? 0), 0),
)
const alertCount = computed(() => world.notifications.length)
</script>

<template>
  <div class="system-stats-bar" data-testid="system-stats-bar">
    <div class="stat" data-testid="stat-research">
      <span class="label">Research</span>
      <span class="value">{{ world.researchTotal.toFixed(1) }} RP</span>
    </div>
    <div class="stat" data-testid="stat-colonies">
      <span class="label">Colonies</span>
      <span class="value">{{ colonyCount }}</span>
    </div>
    <div class="stat" data-testid="stat-population">
      <span class="label">Population</span>
      <span class="value">{{ totalPopulation }}</span>
    </div>
    <div class="stat" data-testid="stat-alerts">
      <span class="label">Alerts</span>
      <span class="value" :class="{ hot: alertCount > 0 }">{{ alertCount }}</span>
    </div>
  </div>
</template>

<style scoped>
.system-stats-bar {
  display: flex;
  align-items: center;
  gap: 1.5rem;
  padding: 0.3rem 1rem;
  background: var(--surface-3);
  border-bottom: 1px solid var(--border-subtle);
  font-size: 0.8rem;
  overflow-x: auto;
  white-space: nowrap;
}
.stat { display: flex; align-items: baseline; gap: 0.4rem; }
.label { color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.05em; font-size: 0.68rem; }
.value { color: var(--accent); font-weight: bold; }
.value.hot { color: var(--warn); }
</style>
