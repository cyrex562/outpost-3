import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

const MAX_EVENTS = 1000

const SEVERITY_RANK = {
  debug: 0,
  info: 1,
  notable: 2,
  critical: 3,
  milestone: 4,
}

export const useEventLogStore = defineStore('eventLog', () => {
  const events = ref([])
  const severityFilter = ref(null) // null = show all

  const filteredEvents = computed(() => {
    if (!severityFilter.value) return events.value
    const minRank = SEVERITY_RANK[severityFilter.value] ?? 0
    return events.value.filter(
      (e) => (SEVERITY_RANK[e.severity] ?? 0) >= minRank
    )
  })

  function addEvent(event) {
    events.value.push({
      ...event,
      _id: events.value.length,
      _ts: Date.now(),
    })
    if (events.value.length > MAX_EVENTS) {
      events.value = events.value.slice(-MAX_EVENTS)
    }
  }

  function clear() {
    events.value = []
  }

  function setSeverityFilter(severity) {
    severityFilter.value = severity
  }

  return {
    events,
    filteredEvents,
    severityFilter,
    addEvent,
    clear,
    setSeverityFilter,
  }
})
