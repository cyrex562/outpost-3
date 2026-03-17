import { ref, computed } from 'vue'
import { defineStore } from 'pinia'

const HISTORY_SIZE = 500 // ring buffer capacity

export const useGameStateStore = defineStore('gameState', () => {
  // ── Current snapshot state ──────────────────────────────────
  const phase = ref(null)
  const resources = ref(null)
  const population = ref(null)
  const shipSystems = ref(null)
  const notables = ref([])
  const starSystems = ref([])
  const transit = ref(null)
  const search = ref(null)
  const survey = ref(null)
  const shipName = ref(null)

  // ── Historical data ring buffer ─────────────────────────────
  // Each entry: { dayOffset, year, resources: {...}, population: {...} }
  const history = ref([])
  let lastHistoryDay = -1

  function updateFromSnapshot(snapshot) {
    phase.value = snapshot.phase
    resources.value = snapshot.resources
    population.value = snapshot.population
    shipSystems.value = snapshot.ship_systems
    notables.value = snapshot.notables || []
    starSystems.value = snapshot.star_systems || []
    transit.value = snapshot.transit
    search.value = snapshot.search
    survey.value = snapshot.survey
    if (snapshot.ship_name) shipName.value = snapshot.ship_name

    // Record history (at most once per game-day to avoid duplication)
    const day = snapshot.day_offset
    if (day > lastHistoryDay && snapshot.resources) {
      lastHistoryDay = day
      const entry = {
        dayOffset: day,
        year: snapshot.year,
        resources: { ...snapshot.resources },
        population: snapshot.population ? snapshot.population.total : null,
        morale: snapshot.population ? snapshot.population.morale : null,
      }

      if (history.value.length >= HISTORY_SIZE) {
        history.value.shift()
      }
      history.value.push(entry)
    }
  }

  // ── Computed helpers ────────────────────────────────────────
  const livingNotables = computed(() =>
    notables.value.filter(n => n.alive)
  )

  const selectedSystem = computed(() =>
    starSystems.value.find(s => s.selected) || null
  )

  const resourceKeys = computed(() => {
    if (!resources.value) return []
    return Object.keys(resources.value)
  })

  // Resource rates (approximate from history)
  const resourceRates = computed(() => {
    const h = history.value
    if (h.length < 2) return {}
    const recent = h[h.length - 1]
    // Look back ~30 days for rate calculation
    const lookback = h.find(e => e.dayOffset <= recent.dayOffset - 30) || h[0]
    const daysDiff = recent.dayOffset - lookback.dayOffset
    if (daysDiff <= 0) return {}

    const rates = {}
    for (const key of Object.keys(recent.resources || {})) {
      const delta = (recent.resources[key] || 0) - (lookback.resources[key] || 0)
      rates[key] = Math.round((delta / daysDiff) * 10) / 10 // per day
    }
    return rates
  })

  return {
    phase,
    resources,
    population,
    shipSystems,
    notables,
    starSystems,
    transit,
    search,
    survey,
    shipName,
    history,
    livingNotables,
    selectedSystem,
    resourceKeys,
    resourceRates,
    updateFromSnapshot,
  }
})
