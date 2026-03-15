import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

const MAX_EVENTS = 500

export const useEventLogStore = defineStore('eventLog', () => {
  const events = ref([])
  const severityFilter = ref(null) // null = show all

  const filteredEvents = computed(() => {
    if (!severityFilter.value) return events.value
    return events.value.filter((e) => e.severity === severityFilter.value)
  })

  function addEvent(event) {
    events.value.push({
      ...event,
      _id: events.value.length,
      _ts: Date.now(),
    })
    // Cap the buffer
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
