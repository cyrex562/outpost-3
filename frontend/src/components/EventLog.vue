<script setup>
import { ref, watch, nextTick } from 'vue'
import { useEventLogStore } from '../stores/eventLog'

const eventLog = useEventLogStore()
const logContainer = ref(null)
const autoScroll = ref(true)
const expandedDeliberations = ref(new Set())

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

function toggleDelib(id) {
  if (expandedDeliberations.value.has(id)) {
    expandedDeliberations.value.delete(id)
  } else {
    expandedDeliberations.value.add(id)
  }
  // Trigger reactivity
  expandedDeliberations.value = new Set(expandedDeliberations.value)
}

function utilityBar(utility) {
  return Math.round(utility * 100) + '%'
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
      <template v-for="event in eventLog.filteredEvents" :key="event._id">

        <!-- Deliberation event — expandable card -->
        <div
          v-if="event.event_type === 'deliberation.complete'"
          class="border border-yellow-800/60 rounded bg-yellow-950/20 overflow-hidden"
        >
          <!-- Collapsed header -->
          <button
            class="w-full flex gap-2 items-start px-2 py-1 text-left hover:bg-yellow-900/10 transition-colors"
            @click="toggleDelib(event._id)"
          >
            <span class="text-panel-muted shrink-0 w-28 text-right tabular-nums text-[10px] mt-0.5">
              Y{{ event.game_time?.year ?? '?' }}.M{{ event.game_time?.month ?? '?' }}.D{{ event.game_time?.day_of_month ?? '?' }}
            </span>
            <span class="flex-1 text-yellow-300 leading-snug">
              ⚖ {{ event.data?.trigger_label }} →
              <span class="font-semibold">{{ event.data?.chosen_label }}</span>
            </span>
            <span class="text-panel-muted text-[10px] shrink-0 mt-0.5">
              {{ expandedDeliberations.has(event._id) ? '▲' : '▼' }}
            </span>
          </button>

          <!-- Expanded detail -->
          <div v-if="expandedDeliberations.has(event._id)" class="px-2 pb-2 space-y-1.5">
            <!-- Options with utility bars and advocate lists -->
            <div
              v-for="opt in event.data?.options"
              :key="opt.key"
              class="space-y-0.5"
              :class="opt.key === event.data?.chosen ? 'opacity-100' : 'opacity-60'"
            >
              <div class="flex items-center gap-2">
                <span class="text-[10px] shrink-0" :class="opt.key === event.data?.chosen ? 'text-yellow-300' : 'text-panel-muted'">
                  {{ opt.key === event.data?.chosen ? '✓' : ' ' }} {{ opt.label }}
                </span>
                <div class="flex-1 h-1 bg-panel-border rounded-full overflow-hidden">
                  <div
                    class="h-full rounded-full"
                    :class="opt.key === event.data?.chosen ? 'bg-yellow-400' : 'bg-panel-muted/40'"
                    :style="{ width: utilityBar(opt.utility) }"
                  />
                </div>
                <span class="text-[10px] text-panel-muted tabular-nums w-8 text-right">{{ Math.round(opt.utility * 100) }}%</span>
              </div>
              <div v-if="opt.advocates?.length" class="text-[10px] text-panel-muted pl-4">
                For: {{ opt.advocates.join(', ') }}
              </div>
            </div>

            <!-- Effect summary -->
            <div class="text-[10px] text-yellow-200/70 border-t border-yellow-800/30 pt-1">
              {{ event.data?.effect_summary }}
            </div>
          </div>
        </div>

        <!-- Standard event row -->
        <div
          v-else
          class="flex gap-2 leading-snug"
          :class="severityColors[event.severity] || 'text-panel-text'"
        >
          <span class="text-panel-muted shrink-0 w-28 text-right tabular-nums">
            Y{{ event.game_time?.year ?? '?' }}.M{{ event.game_time?.month ?? '?' }}.D{{ event.game_time?.day_of_month ?? event.game_time?.day ?? '?' }}
          </span>
          <span>{{ event.text || event.event_type }}</span>
        </div>

      </template>

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
