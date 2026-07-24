<script setup lang="ts">
/**
 * Colonies list (UI-rework PR7) — the "Colonies" nav destination: every
 * founded colony as a card with a vital-stat summary (population, stability,
 * labour, building + in-construction counts). Clicking a card opens that
 * colony's dashboard. Replaces the old "Colony" nav that jumped straight to a
 * single colony.
 */

import { useRouter } from 'vue-router'
import { useWorldStore } from '@/stores/worldStore'

const router = useRouter()
const world = useWorldStore()

function stabilityClass(stability: number): string {
  if (stability > 0.6) return 'stability-high'
  if (stability >= 0.3) return 'stability-mid'
  return 'stability-low'
}

function openColony(id: string): void {
  void router.push({ name: 'colony', params: { colonyId: id } })
}
</script>

<template>
  <div class="colonies-view" data-testid="colonies-view">
    <h2 class="title">Colonies</h2>

    <div v-if="world.colonies.length === 0" class="empty" data-testid="no-colonies">
      No colonies founded yet. Found one from the system map.
    </div>

    <div v-else class="colony-grid">
      <button
        v-for="c in world.colonies"
        :key="c.id"
        class="colony-card"
        :data-testid="`colony-card-${c.id}`"
        @click="openColony(c.id)"
      >
        <div class="card-head">
          <span class="colony-name">{{ c.name }}</span>
          <span class="stability-dot" :class="stabilityClass(c.stability)" :title="`Stability ${(c.stability * 100).toFixed(0)}%`" />
        </div>
        <dl class="summary">
          <div class="stat"><dt>Population</dt><dd data-testid="summary-population">{{ c.population.toFixed(0) }}</dd></div>
          <div class="stat"><dt>Stability</dt><dd :class="stabilityClass(c.stability)">{{ (c.stability * 100).toFixed(0) }}%</dd></div>
          <div class="stat"><dt>Labour</dt><dd>{{ c.available_labour }}</dd></div>
          <div class="stat"><dt>Buildings</dt><dd>{{ c.buildings.length }}</dd></div>
          <div class="stat"><dt>Building</dt><dd>{{ c.active_construction.length }}</dd></div>
        </dl>
      </button>
    </div>
  </div>
</template>

<style scoped>
.colonies-view { display: flex; flex-direction: column; gap: 0.75rem; height: 100%; }
.title { color: #8cf; margin: 0; }
.empty { color: #667; font-style: italic; }

.colony-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 0.75rem;
  align-content: start;
  overflow-y: auto;
}

.colony-card {
  text-align: left;
  background: #12121c;
  border: 1px solid #334;
  border-radius: 6px;
  padding: 0.75rem 0.85rem;
  color: #aab;
  cursor: pointer;
  font-family: monospace;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.colony-card:hover { border-color: #558; background: #161622; }

.card-head { display: flex; align-items: center; justify-content: space-between; }
.colony-name { color: #8cf; font-size: 0.95rem; font-weight: 600; }
.stability-dot { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }

.summary { display: grid; grid-template-columns: 1fr 1fr; gap: 0.25rem 0.75rem; margin: 0; }
.stat { display: flex; justify-content: space-between; gap: 0.5rem; }
.stat dt { color: #667; font-size: 0.72rem; }
.stat dd { color: #cdd; font-size: 0.8rem; margin: 0; }

.stability-high { color: #6adba5; }
.stability-mid  { color: #eab764; }
.stability-low  { color: #e77767; }
.stability-dot.stability-high { background: #4ec990; }
.stability-dot.stability-mid  { background: #d4a24a; }
.stability-dot.stability-low  { background: #d0574a; }
</style>
