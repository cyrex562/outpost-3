/**
 * Composable that owns the WebSocket connection to the Outpost 3 server.
 *
 * Responsibilities:
 * - Manage the connection lifecycle (connect, reconnect on close, disconnect)
 * - Parse inbound server messages and forward them to the world store
 * - Provide a typed `send` helper so callers never construct raw JSON
 *
 * Usage:
 * ```ts
 * const { send, disconnect } = useGameSocket()
 * send({ type: 'command', seq: 1, command: { kind: 'advance_sol' } })
 * ```
 */

import { onUnmounted } from 'vue'
import { useWorldStore } from '@/stores/worldStore'
import type { ClientMessage, ServerMessage } from '@/types/api'
import { isTauri } from '@/services/tauriBridge'

const RECONNECT_DELAY_MS = 2000

/** Return the WebSocket URL relative to the current page origin. */
function wsUrl(): string {
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
  return `${proto}://${window.location.host}/ws`
}

export function useGameSocket() {
  const store = useWorldStore()
  let socket: WebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let destroyed = false

  // Desktop mode: no server to talk to. Report a synthetic 'connected' status
  // so the header UI is happy and skip the WebSocket entirely.
  if (isTauri) {
    store.setConnectionStatus('connected')
    return {
      send: (_msg: ClientMessage): void => {},
      disconnect: (): void => {},
    }
  }

  function connect(): void {
    if (destroyed) return
    store.setConnectionStatus('connecting')
    socket = new WebSocket(wsUrl())

    socket.addEventListener('open', () => {
      store.setConnectionStatus('connected')
    })

    socket.addEventListener('message', (ev: MessageEvent<string>) => {
      let msg: ServerMessage
      try {
        msg = JSON.parse(ev.data) as ServerMessage
      } catch {
        return
      }
      store.handleServerMessage(msg)
    })

    socket.addEventListener('close', () => {
      store.setConnectionStatus('disconnected')
      if (!destroyed) {
        reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS)
      }
    })

    socket.addEventListener('error', () => {
      store.setConnectionStatus('error')
    })
  }

  function disconnect(): void {
    destroyed = true
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    socket?.close()
    socket = null
    store.setConnectionStatus('disconnected')
  }

  /**
   * Send a typed client message.
   * Silently drops the message if the socket is not open.
   */
  function send(msg: ClientMessage): void {
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify(msg))
    }
  }

  onUnmounted(disconnect)

  connect()

  return { send, disconnect }
}
