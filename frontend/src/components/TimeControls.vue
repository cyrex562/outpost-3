<script setup>
import { useTimeStore } from '../stores/time'

const time = useTimeStore()

const speeds = [
  { value: 1, label: '1×' },
  { value: 5, label: '5×' },
  { value: 25, label: '25×' },
  { value: 100, label: '100×' },
  { value: 0, label: 'MAX' },
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
        Day {{ time.day }}
      </span>
      <span
        class="ml-auto text-[10px] px-1 rounded"
        :class="time.connected ? 'text-green-400 bg-green-900/30' : 'text-red-400 bg-red-900/30'"
      >
        {{ time.connected ? 'LIVE' : 'OFFLINE' }}
      </span>
    </div>

    <!-- Controls row -->
    <div class="flex items-center gap-2">
      <!-- Play/Pause -->
      <button
        class="px-2 py-1 rounded border border-panel-border hover:border-panel-accent transition-colors"
        :class="time.paused ? 'text-green-400' : 'text-panel-accent'"
        @click="time.togglePause()"
      >
        {{ time.paused ? '▶ Play' : '⏸ Pause' }}
      </button>

      <!-- Speed buttons -->
      <div class="flex gap-1">
        <button
          v-for="s in speeds"
          :key="s.value"
          class="px-1.5 py-0.5 rounded border transition-colors"
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
  </div>
</template>
