<script setup>
import { useTimeStore } from '../stores/time'

const time = useTimeStore()

const speeds = [
  { level: 1,  label: '1d' },
  { level: 2,  label: '2d' },
  { level: 3,  label: '5d' },
  { level: 4,  label: '10d' },
  { level: 5,  label: '25d' },
  { level: 6,  label: '50d' },
  { level: 7,  label: '100d' },
  { level: 8,  label: '250d' },
  { level: 9,  label: '500d' },
  { level: 10, label: '1Kd' },
  { level: 11, label: '2.5Kd' },
  { level: 12, label: '5Kd' },
  { level: 13, label: '10Kd' },
  { level: 14, label: '25Kd' },
  { level: 15, label: 'MAX' },
]

function currentLabel() {
  return speeds.find(s => s.level === time.speed)?.label || time.speed
}
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

    <!-- Play/Pause + Speed label -->
    <div class="flex items-center gap-2">
      <button
        class="px-2 py-1 rounded border border-panel-border hover:border-panel-accent transition-colors"
        :class="time.paused ? 'text-green-400' : 'text-panel-accent'"
        @click="time.togglePause()"
      >
        {{ time.paused ? '▶ Play' : '⏸ Pause' }}
      </button>
      <span class="text-panel-muted text-[10px]">
        Speed: {{ currentLabel() }} / 2.5s
      </span>
    </div>

    <!-- Speed slider -->
    <div class="flex flex-col gap-1">
      <input
        type="range"
        min="1"
        max="15"
        :value="time.speed"
        @input="time.setSpeed(Number($event.target.value))"
        class="w-full accent-panel-accent cursor-pointer"
      />
      <div class="flex justify-between text-[9px] text-panel-muted">
        <span>1d</span>
        <span>100d</span>
        <span>MAX</span>
      </div>
    </div>

    <!-- Speed preset buttons (compact) -->
    <div class="grid grid-cols-5 gap-1">
      <button
        v-for="s in speeds"
        :key="s.level"
        class="px-0.5 py-0.5 rounded border text-center transition-colors text-[9px]"
        :class="
          time.speed === s.level
            ? 'border-panel-accent text-panel-accent bg-panel-accent/10'
            : 'border-panel-border text-panel-muted hover:border-panel-text'
        "
        @click="time.setSpeed(s.level)"
      >
        {{ s.label }}
      </button>
    </div>
  </div>
</template>
