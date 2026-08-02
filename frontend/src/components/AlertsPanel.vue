<script setup lang="ts">
/**
 * Colony log (events/alerts unification): a single chronological list —
 * previously two stacked sections (a curated "Alerts" list and a separate
 * raw "Event Log"). Every server event gets exactly one row here, newest
 * first; alert-tier ones (tier != 'ambient') are colored to stand out and
 * already popped an `AlertToast` when they arrived (see `worldStore`'s
 * `alertToast`) — everything else is still logged, just in the muted
 * ambient color. Click a row to expand its full raw event payload — a
 * generic key/value dump rather than a bespoke template per event kind, so
 * every one of the ~25 event kinds gets a detail view for free.
 */

import { ref } from 'vue'
import type { LogEntry } from '@/worldModel/model'
import type { ServerEvent } from '@/types/events'

const props = defineProps<{
  logEntries: LogEntry[]
}>()

const emit = defineEmits<{
  (e: 'clear-log'): void
}>()

function formatEventKind(kind: string): string {
  return kind.replace(/_/g, ' ')
}

/** Every field on the raw event except `kind` and `colony_id` (shown as the
 * row's own label and its own dedicated detail row respectively), formatted
 * for a plain key/value detail dump. */
function eventDetailRows(event: ServerEvent): { key: string; value: string }[] {
  return Object.entries(event)
    .filter(([key]) => key !== 'kind' && key !== 'colony_id')
    .map(([key, value]) => ({
      key,
      value: typeof value === 'object' && value !== null ? JSON.stringify(value) : String(value),
    }))
}

/** The event's own `colony_id`, if this event kind carries one — used as a
 * fallback for ambient rows, which don't get a curated `entry.colony_id`. */
function eventColonyId(event: ServerEvent): string | undefined {
  return 'colony_id' in event ? (event as unknown as { colony_id: string }).colony_id : undefined
}

/** Rows currently expanded to show their detail dump — a plain Set,
 * reassigned wholesale on toggle (matches this codebase's established
 * convention for reactive Set state, e.g. ColonyView's `cancelingIds`). */
const expandedIds = ref<Set<string>>(new Set())

function toggle(id: string): void {
  const next = new Set(expandedIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  expandedIds.value = next
}
</script>

<template>
  <div class="panel" data-testid="alerts-panel">
    <div class="log-header">
      <h4 class="panel-title">Log</h4>
      <button class="btn-clear-log" data-testid="btn-clear-log" @click="emit('clear-log')">Clear</button>
    </div>
    <div v-if="props.logEntries.length === 0" class="hint" data-testid="log-empty">No log entries yet.</div>
    <ul v-else class="log-list" data-testid="log-list">
      <li
        v-for="entry in [...props.logEntries].reverse()"
        :key="entry.id"
        class="log-item"
        :class="[`tier-${entry.tier}`, { expanded: expandedIds.has(entry.id) }]"
        :data-testid="`log-item-${entry.event.kind}`"
        @click="toggle(entry.id)"
      >
        <div class="log-row">
          <span class="log-sol">sol {{ entry.timestamp_sol }}</span>
          <span class="log-message">{{ entry.tier !== 'ambient' ? entry.message : formatEventKind(entry.event.kind) }}</span>
        </div>
        <dl v-if="expandedIds.has(entry.id)" class="log-detail" :data-testid="`log-detail-${entry.event.kind}`">
          <div class="detail-row">
            <dt>kind</dt>
            <dd>{{ entry.event.kind }}</dd>
          </div>
          <div v-if="entry.colony_id || eventColonyId(entry.event)" class="detail-row">
            <dt>colony_id</dt>
            <dd>{{ entry.colony_id ?? eventColonyId(entry.event) }}</dd>
          </div>
          <div v-for="row in eventDetailRows(entry.event)" :key="row.key" class="detail-row">
            <dt>{{ row.key }}</dt>
            <dd>{{ row.value }}</dd>
          </div>
        </dl>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: #8cf; font-size: 0.9rem; margin: 0; }
.log-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem; }
.hint { font-size: 0.75rem; color: #446; font-style: italic; }

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

.log-list { list-style: none; display: flex; flex-direction: column; gap: 0.15rem; margin: 0; padding: 0; }
.log-item {
  font-size: 0.76rem;
  padding: 0.25rem 0.45rem;
  border-left: 3px solid #334;
  color: #778;
  cursor: pointer;
  border-radius: 2px;
}
.log-item:hover { background: rgba(255, 255, 255, 0.03); }
.log-item.expanded { background: rgba(255, 255, 255, 0.04); }

.log-row { display: flex; align-items: baseline; gap: 0.5rem; }
.log-sol { color: #556; font-size: 0.68rem; flex-shrink: 0; }
.log-message { min-width: 0; overflow-wrap: anywhere; }

/* Tier coloring is the single source of truth for "is this an alert" now
   that there's one list instead of two — matches the tones the old
   .notification.tier-* classes used, so returning players see the same
   color language. */
.log-item.tier-blocking { border-color: #f44; color: #f66; }
.log-item.tier-urgent   { border-color: #c44; color: #c66; }
.log-item.tier-notable  { border-color: #c84; color: #ca8; }
.log-item.tier-ambient  { border-color: #334; color: #778; }

.log-detail {
  margin: 0.35rem 0 0.15rem;
  padding: 0.35rem 0.5rem;
  background: #0d0d15;
  border: 1px solid #223;
  border-radius: 3px;
  font-size: 0.7rem;
}
.detail-row { display: flex; gap: 0.5rem; padding: 0.08rem 0; }
.detail-row dt { color: #556; flex-shrink: 0; min-width: 9rem; }
.detail-row dd { margin: 0; color: #99a; overflow-wrap: anywhere; }
</style>
