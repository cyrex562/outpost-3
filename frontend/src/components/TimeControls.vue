<script setup>
import { useTimeStore } from '../stores/time'

const time = useTimeStore()

const speeds = [
  { value: 1, label: '1d' },
  { value: 2, label: '2d' },
  { value: 7, label: '1w' },
  { value: 10, label: '10d' },
  { value: 30, label: '1mo' },
  { value: 90, label: '3mo' },
  { value: 180, label: '6mo' },
  { value: 365, label: '1y' },
  { value: 730, label: '2y' },
  { value: 1825, label: '5y' },
  { value: 3650, label: '10y' },
  { value: 9125, label: '25y' },
  { value: 18250, label: '50y' },
  { value: 36500, label: '100y' },
]
</script>

<template>
  <div class="flex flex-col gap-2 text-xs">
    <!-- Clock display -->
    <div class="flex items-baseline gap-2">
      <span class="text-lg font-semibold text-panel-text">
        Year {{ time.year }}
      </span>
      <span class="text-panel-muted">
        M{{ time.month }} D{{ time.dayOfMonth }}
      </span>
      <span
        class="ml-auto text-[10px] px-1 rounded"
        :class="time.connected ? 'text-green-400 bg-green-900/30' : 'text-red-400 bg-red-900/30'"
      >
        {{ time.connected ? 'LIVE' : 'OFFLINE' }}
      </span>
    </div>

    <!-- Play/Pause -->
    <div class="flex items-center gap-2">
      <button
        class="px-2 py-1 rounded border border-panel-border hover:border-panel-accent transition-colors"
        :class="time.paused ? 'text-green-400' : 'text-panel-accent'"
        @click="time.togglePause()"
      >
        {{ time.paused ? '▶ Play' : '⏸ Pause' }}
      </button>
      <span class="text-panel-muted text-[10px]">
        Speed: {{ speeds.find(s => s.value === time.speed)?.label || time.speed }}
      </span>
    </div>

    <!-- Speed grid -->
    <div class="grid grid-cols-7 gap-1">
      <button
        v-for="s in speeds"
        :key="s.value"
        class="px-1 py-0.5 rounded border text-center transition-colors"
        :class="
          time.speed === s.value
            ? 'border-panel-accent text-panel-accent bg-panel-accent/10'
            : 'border-panel-border text-panel-muted hover:border-panel-text'
        "
        @click="time.setSpeed(s.value)"
      >
        {{ s.label }}
      </button>
    </div>
  </div>
</template>
