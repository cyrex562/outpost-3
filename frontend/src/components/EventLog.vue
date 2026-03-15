<script setup>
import { ref, watch, nextTick } from 'vue'
import { useEventLogStore } from '../stores/eventLog'

const eventLog = useEventLogStore()
const logContainer = ref(null)
const autoScroll = ref(true)

const severityColors = {
  debug: 'text-panel-muted',
  info: 'text-panel-text',
  notable: 'text-yellow-400',
  critical: 'text-orange-400',
  milestone: 'text-panel-accent font-semibold',
}

const severityOptions = [
  { value: null, label: 'All' },
  { value: 'info', label: 'Info+' },
  { value: 'notable', label: 'Notable+' },
  { value: 'critical', label: 'Critical+' },
  { value: 'milestone', label: 'Milestones' },
]

// Auto-scroll to bottom when new events arrive
watch(
  () => eventLog.filteredEvents.length,
  async () => {
    if (!autoScroll.value) return
    await nextTick()
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  }
)

function onScroll() {
  if (!logContainer.value) return
  const el = logContainer.value
  // Disable auto-scroll if user scrolls up, re-enable if at bottom
  const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 30
  autoScroll.value = atBottom
}
</script>

<template>
  <div class="flex flex-col h-full gap-1 text-xs">
    <!-- Filter bar -->
    <div class="flex items-center gap-1 shrink-0">
      <button
        v-for="opt in severityOptions"
        :key="opt.label"
        class="px-1.5 py-0.5 rounded border transition-colors"
        :class="
          eventLog.severityFilter === opt.value
            ? 'border-panel-accent text-panel-accent bg-panel-accent/10'
            : 'border-panel-border text-panel-muted hover:border-panel-text'
        "
        @click="eventLog.setSeverityFilter(opt.value)"
      >
        {{ opt.label }}
      </button>
      <button
        class="ml-auto px-1.5 py-0.5 rounded border border-panel-border text-panel-muted hover:text-panel-accent hover:border-panel-accent transition-colors"
        @click="eventLog.clear()"
      >
        Clear
      </button>
    </div>

    <!-- Event list -->
    <div
      ref="logContainer"
      class="flex-1 overflow-y-auto space-y-0.5 min-h-0"
      @scroll="onScroll"
    >
      <div
        v-for="event in eventLog.filteredEvents"
        :key="event._id"
        class="flex gap-2 leading-snug"
        :class="severityColors[event.severity] || 'text-panel-text'"
      >
        <span class="text-panel-muted shrink-0 w-28 text-right tabular-nums">
          Y{{ event.game_time?.year ?? '?' }}.M{{ event.game_time?.month ?? '?' }}.D{{ event.game_time?.day_of_month ?? event.game_time?.day ?? '?' }}
        </span>
        <span>{{ event.text || event.event_type }}</span>
      </div>

      <div
        v-if="eventLog.filteredEvents.length === 0"
        class="text-panel-muted italic py-4 text-center"
      >
        No events yet. Press Play to start the simulation.
      </div>
    </div>

    <!-- Auto-scroll indicator -->
    <div v-if="!autoScroll" class="shrink-0 text-center">
      <button
        class="text-[10px] text-panel-muted hover:text-panel-accent"
        @click="autoScroll = true"
      >
        ↓ Resume auto-scroll
      </button>
    </div>
  </div>
</template>
