<script setup lang="ts">
/**
 * Vital statistics panel (UI-rework PR4) — the colony's at-a-glance vitals in
 * a panel of their own: headcount + growth-trend sparkline, stability bar,
 * available labour, and build-slot usage. Pure display; no commands
 * originate here. (Evolved from the former PopulationPanel, which this
 * replaces, folding in the build-slot summary that used to live inside
 * BuildingsPanel.)
 */

const props = defineProps<{
  population: number
  stability: number
  availableLabour: number
  /** Recent population samples, oldest first (indicative trend only). */
  populationTrend: number[]
  /** Build slots currently reserved by buildings / in-flight projects. */
  slotsUsed: number
  /** Total build-slot capacity for the colony. */
  slotCapacity: number
}>()

function stabilityClass(stability: number): string {
  if (stability > 0.6) return 'stability-high'
  if (stability >= 0.3) return 'stability-mid'
  return 'stability-low'
}

function stabilityLabel(stability: number): string {
  const pct = (stability * 100).toFixed(0)
  if (stability > 0.6) return `${pct}% — Stable`
  if (stability >= 0.3) return `${pct}% — Uncertain`
  return `${pct}% — Critical`
}

function sparklinePath(values: number[]): string {
  if (values.length < 2) return ''
  const min = Math.min(...values)
  const max = Math.max(...values)
  const range = max - min || 1
  const w = 80
  const h = 20
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * w
    const y = h - ((v - min) / range) * h
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  return `M ${pts.join(' L ')}`
}
</script>

<template>
  <div class="panel" data-testid="vital-stats-panel">
    <h4 class="panel-title">Vital Statistics</h4>

    <div class="stat-row" data-testid="population-section">
      <div class="stat-block">
        <span class="stat-label">Population</span>
        <span class="stat-value" data-testid="population-count">
          {{ props.population.toFixed(0) }}
        </span>
      </div>
      <div class="sparkline-wrap" title="Population trend (last sessions)">
        <svg width="80" height="20" class="sparkline" data-testid="population-sparkline" aria-hidden="true">
          <path
            v-if="props.populationTrend.length >= 2"
            :d="sparklinePath(props.populationTrend)"
            fill="none"
            stroke="#4c8"
            stroke-width="1.5"
          />
          <line
            v-else
            x1="0" y1="10" x2="80" y2="10"
            stroke="#444"
            stroke-width="1"
            stroke-dasharray="4 3"
          />
        </svg>
      </div>
    </div>

    <div class="stat-block stability-section" data-testid="stability-section">
      <span class="stat-label">Stability</span>
      <div
        class="stability-bar-track"
        role="progressbar"
        :aria-valuenow="Math.round(props.stability * 100)"
        aria-valuemin="0"
        aria-valuemax="100"
        data-testid="stability-bar"
      >
        <div
          class="stability-bar-fill"
          :class="stabilityClass(props.stability)"
          :style="{ width: `${(props.stability * 100).toFixed(1)}%` }"
        />
      </div>
      <span class="stability-label" :class="stabilityClass(props.stability)" data-testid="stability-label">
        {{ stabilityLabel(props.stability) }}
      </span>
    </div>

    <div class="stat-row">
      <div class="stat-block">
        <span class="stat-label">Labour</span>
        <span class="stat-value" data-testid="labour-available">{{ props.availableLabour }}</span>
      </div>
      <div class="stat-block">
        <span class="stat-label">Build slots</span>
        <span class="stat-value" data-testid="build-slots">{{ props.slotsUsed }} / {{ props.slotCapacity }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: #8cf; font-size: 0.9rem; margin: 0 0 0.6rem; }

.stat-row { display: flex; gap: 1.5rem; margin-bottom: 0.5rem; align-items: flex-end; }
.stat-block { display: flex; flex-direction: column; }
.stat-label { font-size: 0.7rem; color: #668; margin-bottom: 0.1rem; }
.stat-value { font-size: 1.1rem; color: #dde; }

.sparkline-wrap { align-self: flex-end; }
.sparkline { display: block; }

.stability-section { margin-bottom: 0.75rem; }
.stability-bar-track {
  height: 12px;
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 4px;
  overflow: hidden;
  margin: 0.35rem 0;
  width: 100%;
  max-width: 300px;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.45);
}
.stability-bar-fill { height: 100%; transition: width 0.3s ease, background 0.2s; }
.stability-bar-fill.stability-high { background: #4ec990; }
.stability-bar-fill.stability-mid  { background: #d4a24a; }
.stability-bar-fill.stability-low  { background: #d0574a; }

.stability-label { font-size: 0.78rem; font-weight: 600; letter-spacing: 0.02em; }
.stability-label.stability-high { color: #6adba5; }
.stability-label.stability-mid  { color: #eab764; }
.stability-label.stability-low  { color: #e77767; }
</style>
