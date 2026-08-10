<script setup lang="ts">
/**
 * Vital statistics panel (UI-rework PR4) — the colony's at-a-glance vitals in
 * a panel of their own: headcount + growth-trend sparkline, stability bar,
 * available labour, and build-slot usage. Pure display; no commands
 * originate here. (Evolved from the former PopulationPanel, which this
 * replaces, folding in the build-slot summary that used to live inside
 * BuildingsPanel.)
 */

import { computed } from 'vue'

const props = defineProps<{
  population: number
  stability: number
  /** Morale scalar in [0, 1] (issue #382) — separate from stability, see
   * `crate::morale`'s module doc comment in the engine. */
  morale: number
  /** Colony-wide output multiplier and its cause (issue #444). */
  productivityModifier?: number
  productivityNote?: string | null
  availableLabour: number
  /** Recent population samples, oldest first (indicative trend only). */
  populationTrend: number[]
  /** Build slots currently reserved by buildings / in-flight projects. */
  slotsUsed: number
  /** Total build-slot capacity for the colony. */
  slotCapacity: number
  /** Workforce taken up by jobs at operational buildings (issue #305). */
  labourEmployed: number
  /** Workforce with no job to go to (issue #305). */
  labourUnemployed: number
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

// Morale reuses stability's bucketing/color scheme — same three-band read at
// a glance, applied to a different (quality-of-life, not survival) scalar.
/** Only worth a row when the world actually changes output (issue #444). */
const showProductivity = computed(
  () =>
    props.productivityModifier !== undefined &&
    Math.abs(props.productivityModifier - 1) > 0.005,
)

function moraleClass(morale: number): string {
  if (morale > 0.6) return 'stability-high'
  if (morale >= 0.3) return 'stability-mid'
  return 'stability-low'
}

function moraleLabel(morale: number): string {
  const pct = (morale * 100).toFixed(0)
  if (morale > 0.6) return `${pct}% — Content`
  if (morale >= 0.3) return `${pct}% — Restless`
  return `${pct}% — Miserable`
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
            class="sparkline-trend"
            stroke-width="1.5"
          />
          <line
            v-else
            x1="0" y1="10" x2="80" y2="10"
            class="sparkline-empty"
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

    <!-- Issue #444: every building's output is scaled by this, and nothing
         used to say so — a colony on a hostile world simply produced less
         than the same buildings elsewhere. Shown only when it is off neutral,
         so an ordinary colony isn't told "x1.00, nothing is wrong". -->
    <div
      v-if="showProductivity"
      class="stat-block"
      data-testid="productivity-section"
      :title="props.productivityNote ?? ''"
    >
      <span class="stat-label">Output</span>
      <span class="stat-value" data-testid="productivity-value">
        ×{{ (props.productivityModifier ?? 1).toFixed(2) }}
      </span>
      <span v-if="props.productivityNote" class="hint" data-testid="productivity-note">
        {{ props.productivityNote }}
      </span>
    </div>

    <div class="stat-block stability-section" data-testid="morale-section">
      <span class="stat-label">Morale</span>
      <div
        class="stability-bar-track"
        role="progressbar"
        :aria-valuenow="Math.round(props.morale * 100)"
        aria-valuemin="0"
        aria-valuemax="100"
        data-testid="morale-bar"
      >
        <div
          class="stability-bar-fill"
          :class="moraleClass(props.morale)"
          :style="{ width: `${(props.morale * 100).toFixed(1)}%` }"
        />
      </div>
      <span class="stability-label" :class="moraleClass(props.morale)" data-testid="morale-label">
        {{ moraleLabel(props.morale) }}
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

    <!-- Employed vs unemployed split of the available workforce (#305). -->
    <div class="stat-row" data-testid="labour-breakdown">
      <div class="stat-block">
        <span class="stat-label">Employed</span>
        <span class="stat-value" data-testid="labour-employed">
          {{ props.labourEmployed.toFixed(0) }}
        </span>
      </div>
      <div class="stat-block">
        <span class="stat-label">Unemployed</span>
        <span
          class="stat-value"
          :class="{ 'stat-warn': props.labourUnemployed > 0 }"
          data-testid="labour-unemployed"
        >
          {{ props.labourUnemployed.toFixed(0) }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Inline SVG paint attributes can't read custom properties — the sparkline's
   trend line and its no-data placeholder carry classes so both follow the
   theme. */
.sparkline-trend { stroke: var(--status-good); }
.sparkline-empty { stroke: var(--border-strong); }

.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: var(--accent); font-size: 0.9rem; margin: 0 0 0.6rem; }

.stat-row { display: flex; gap: 1.5rem; margin-bottom: 0.5rem; align-items: flex-end; }
.stat-block { display: flex; flex-direction: column; }
.stat-label { font-size: 0.7rem; color: var(--text-dim); margin-bottom: 0.1rem; }
.stat-value { font-size: 1.1rem; color: var(--text-bright); }
.stat-value.stat-warn { color: var(--status-warn); }

.sparkline-wrap { align-self: flex-end; }
.sparkline { display: block; }

.stability-section { margin-bottom: 0.75rem; }
.stability-bar-track {
  height: 12px;
  background: var(--surface-3);
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
  margin: 0.35rem 0;
  width: 100%;
  max-width: 300px;
  box-shadow: inset 0 1px 2px var(--shadow-soft);
}
.stability-bar-fill { height: 100%; transition: width 0.3s ease, background 0.2s; }
.stability-bar-fill.stability-high { background: var(--status-good); }
.stability-bar-fill.stability-mid  { background: var(--status-warn); }
.stability-bar-fill.stability-low  { background: var(--status-bad); }

.stability-label { font-size: 0.78rem; font-weight: 600; letter-spacing: 0.02em; }
.stability-label.stability-high { color: var(--status-good); }
.stability-label.stability-mid  { color: var(--status-warn); }
.stability-label.stability-low  { color: var(--status-bad); }
</style>
