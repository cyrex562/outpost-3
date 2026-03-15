import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useTimeStore = defineStore('time', () => {
  const year = ref(1)
  const month = ref(1)
  const day = ref(1)
  const dayOfMonth = ref(1)
  const dayOffset = ref(0)
  const speed = ref(1)
  const paused = ref(true)
  const connected = ref(false)

  function updateFromServer(state) {
    year.value = state.year ?? year.value
    month.value = state.month ?? month.value
    day.value = state.day ?? day.value
    dayOfMonth.value = state.day_of_month ?? dayOfMonth.value
    dayOffset.value = state.day_offset ?? dayOffset.value
    speed.value = state.speed ?? speed.value
    paused.value = state.paused ?? paused.value
  }

  async function sendCommand(endpoint) {
    try {
      await fetch(endpoint, { method: 'POST' })
    } catch (err) {
      console.error('Command failed:', err)
    }
  }

  const pause = () => sendCommand('/api/pause')
  const resume = () => sendCommand('/api/resume')
  const togglePause = () => (paused.value ? resume() : pause())
  const setSpeed = (s) => sendCommand(`/api/speed/${s}`)

  return {
    year,
    month,
    day,
    dayOfMonth,
    dayOffset,
    speed,
    paused,
    connected,
    updateFromServer,
    pause,
    resume,
    togglePause,
    setSpeed,
  }
})
