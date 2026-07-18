/**
 * Composable that owns the WebSocket connection to the Outpost 3 server.
 *
 * Responsibilities:
 * - Manage the connection lifecycle (connect, reconnect on close, disconnect)
 * - Parse inbound server messages and forward them to the world store
 * - Provide a typed `send` helper so callers never construct raw JSON
 *
 * The connection is a module-level singleton, not per-call state: several
 * components call `useGameSocket()` independently (`App.vue` at the app
 * root, `NewGameView.vue` to send the `new_game` command). Before issue
 * #243 each call opened its *own* `WebSocket`, so navigating to `/new-game`
 * and back created a second, independent connection — both received every
 * broadcasted event (double-applying them to the world store) and
 * `NewGameView`'s `onUnmounted` teardown clobbered the shared
 * `connectionStatus` to `'disconnected'` even while `App.vue`'s own
 * connection was still alive. Singleton state fixes both: every caller
 * shares the one real connection, and only the first caller's `connect()`
 * actually opens a socket.
 *
 * Usage:
 * ```ts
 * const { send, disconnect } = useGameSocket()
 * send({ type: 'command', seq: 1, command: { kind: 'advance_sol' } })
 * ```
 */

import { useWorldStore } from '@/stores/worldStore'
import type { ClientMessage, ServerMessage } from '@/types/api'
import { isTauri } from '@/services/tauriBridge'

const RECONNECT_DELAY_MS = 2000

/** Return the WebSocket URL relative to the current page origin. */
function wsUrl(): string {
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
  return `${proto}://${window.location.host}/ws`
}

let socket: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let destroyed = false
let connecting = false

// Cached once, on the first `useGameSocket()` call from within a component's
// setup — `connect()` may later run from an async `setTimeout` reconnect
// callback with no active Pinia context, so it must not call `useWorldStore()`
// itself.
let cachedStore: ReturnType<typeof useWorldStore> | null = null

function connect(): void {
  if (destroyed || connecting || socket?.readyState === WebSocket.OPEN) return
  connecting = true
  const store = cachedStore as ReturnType<typeof useWorldStore>
  store.setConnectionStatus('connecting')
  socket = new WebSocket(wsUrl())

  socket.addEventListener('open', () => {
    connecting = false
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
    connecting = false
    store.setConnectionStatus('disconnected')
    if (!destroyed) {
      reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS)
    }
  })

  socket.addEventListener('error', () => {
    connecting = false
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
  cachedStore?.setConnectionStatus('disconnected')
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

export function useGameSocket() {
  const store = useWorldStore()

  // Desktop mode: no server to talk to. Report a synthetic 'connected' status
  // so the header UI is happy and skip the WebSocket entirely.
  if (isTauri) {
    store.setConnectionStatus('connected')
    return {
      send: (_msg: ClientMessage): void => {},
      disconnect: (): void => {},
    }
  }

  cachedStore ??= store
  connect()

  return { send, disconnect }
}
