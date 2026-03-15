import { useTimeStore } from '../stores/time'
import { useEventLogStore } from '../stores/eventLog'

let ws = null
let reconnectTimer = null

export function useWebSocket() {
  function connect() {
    const timeStore = useTimeStore()
    const eventLog = useEventLogStore()

    // Build WebSocket URL relative to current host
    const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
    const url = `${proto}//${location.host}/ws`

    ws = new WebSocket(url)

    ws.onopen = () => {
      console.log('[ws] connected')
      timeStore.connected = true
      if (reconnectTimer) {
        clearTimeout(reconnectTimer)
        reconnectTimer = null
      }
    }

    ws.onmessage = (msg) => {
      try {
        const data = JSON.parse(msg.data)

        if (data.type === 'state') {
          timeStore.updateFromServer(data)
        } else if (data.type === 'event') {
          eventLog.addEvent(data)
        }
      } catch (err) {
        console.error('[ws] parse error:', err)
      }
    }

    ws.onclose = () => {
      console.log('[ws] disconnected, reconnecting in 2s...')
      timeStore.connected = false
      reconnectTimer = setTimeout(connect, 2000)
    }

    ws.onerror = (err) => {
      console.error('[ws] error:', err)
      ws.close()
    }
  }

  return { connect }
}
