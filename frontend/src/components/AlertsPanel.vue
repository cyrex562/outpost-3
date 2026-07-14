<script setup lang="ts">
/**
 * Alerts panel: player-facing notifications plus the raw event log,
 * promoted from an always-visible sidebar strip into its own dockable
 * panel (issue #169).
 */

import type { Notification } from '@/worldModel/model'
import type { ServerEvent } from '@/types/events'

const props = defineProps<{
  notifications: Notification[]
  eventLog: ServerEvent[]
}>()

const emit = defineEmits<{
  (e: 'clear-log'): void
}>()

function formatEventKind(kind: string): string {
  return kind.replace(/_/g, ' ')
}

function eventLogClass(kind: string): string {
  if (kind.includes('shortfall') || kind.includes('cancel')) return 'log-warn'
  if (kind.includes('founded') || kind.includes('constructed') || kind.includes('advanced')) return 'log-info'
  return 'log-default'
}
</script>

<template>
  <div class="panel" data-testid="alerts-panel">
    <div v-if="props.notifications.length > 0" class="notifications" data-testid="notifications">
      <h4 class="panel-title">Alerts</h4>
      <ul>
        <li v-for="n in props.notifications" :key="n.id" :class="`notification tier-${n.tier}`">
          {{ n.message }}
        </li>
      </ul>
    </div>
    <div v-else class="panel-title-row">
      <h4 class="panel-title">Alerts</h4>
      <span class="hint">No alerts yet.</span>
    </div>

    <div class="event-log" data-testid="event-log">
      <div class="event-log-header">
        <h4 class="panel-title" style="margin: 0">Event Log</h4>
        <button class="btn-clear-log" data-testid="btn-clear-event-log" @click="emit('clear-log')">Clear</button>
      </div>
      <div v-if="props.eventLog.length === 0" class="hint" style="padding: 0.4rem 0">No events yet.</div>
      <ul v-else class="log-list" data-testid="event-log-list">
        <li
          v-for="(ev, idx) in [...props.eventLog].reverse()"
          :key="idx"
          class="log-item"
          :class="eventLogClass(ev.kind)"
          :data-testid="`log-item-${ev.kind}`"
        >
          {{ formatEventKind(ev.kind) }}
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: #8cf; font-size: 0.9rem; margin: 0 0 0.6rem; }
.panel-title-row { display: flex; align-items: baseline; gap: 0.5rem; margin-bottom: 0.6rem; }
.hint { font-size: 0.75rem; color: #446; font-style: italic; }

.notifications ul { list-style: none; margin: 0; padding: 0; }
.notification { padding: 0.2rem 0.5rem; font-size: 0.78rem; border-left: 3px solid #555; margin-bottom: 0.2rem; }
.notification.tier-notable  { border-color: #c84; color: #ca8; }
.notification.tier-urgent   { border-color: #c44; color: #c66; }
.notification.tier-blocking { border-color: #f44; color: #f66; }
.notification.tier-ambient  { border-color: #446; color: #778; }

.event-log { margin-top: 1rem; background: #111118; border: 1px solid #334; border-radius: 4px; padding: 0.75rem; }
.event-log-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.4rem; }
.btn-clear-log {
  background: transparent;
  border: 1px solid #334;
  border-radius: 3px;
  color: #556;
  padding: 0.1rem 0.4rem;
  font-size: 0.7rem;
  cursor: pointer;
}
.btn-clear-log:hover { color: #889; border-color: #446; }
.log-list { list-style: none; max-height: 200px; overflow-y: auto; display: flex; flex-direction: column; gap: 0.15rem; margin: 0; padding: 0; }
.log-item { font-size: 0.74rem; padding: 0.15rem 0.35rem; border-left: 2px solid #334; color: #778; }
.log-info    { border-color: #468; color: #8ab; }
.log-warn    { border-color: #853; color: #b86; }
.log-default { border-color: #334; color: #667; }
</style>
