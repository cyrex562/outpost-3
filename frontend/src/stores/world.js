import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useWorldStore = defineStore('world', () => {
  const phase = ref('unknown')
  const ship = ref({ name: '', integrity: 100 })
  const population = ref({ count: 0, births_this_year: 0, deaths_this_year: 0 })
  const resources = ref({ food: 0, water: 0, medicine: 0, fuel: 0, spare_parts: 0 })
  const notables = ref([])
  const loaded = ref(false)

  function updateFromServer(data) {
    if (data.phase !== undefined) phase.value = data.phase
    if (data.ship !== undefined) ship.value = data.ship
    if (data.population !== undefined) population.value = data.population
    if (data.resources !== undefined) resources.value = data.resources
    if (data.notables !== undefined) notables.value = data.notables
    loaded.value = true
  }

  return { phase, ship, population, resources, notables, loaded, updateFromServer }
})
