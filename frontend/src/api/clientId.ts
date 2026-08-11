/**
 * Stable per-tab client id (issue #452).
 *
 * The browser-mode server delivers events two ways: `POST /api/command`
 * returns the events its command produced, and the same events are broadcast
 * over the WebSocket to every connected client. The issuing tab is both, so it
 * used to receive — and log — every command-issued event twice.
 *
 * Sending this id on both transports lets the server skip the echo: the tab
 * applies the events from its own command response straight away, and the
 * broadcast it would otherwise have received back is suppressed. Other tabs
 * still get everything.
 *
 * Per *tab*, not per browser: `sessionStorage` rather than `localStorage`, so
 * two tabs of the same game are genuinely distinct clients and each still sees
 * the other's commands. It survives a reload, which matters because the socket
 * reconnects with the same id.
 */
const STORAGE_KEY = 'outpost3.clientId'

function generate(): string {
  // `randomUUID` needs a secure context; plain HTTP on localhost qualifies,
  // but a LAN host over http:// does not — hence the fallback.
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID()
  }
  return `c-${Math.random().toString(36).slice(2)}-${Date.now().toString(36)}`
}

let cached: string | null = null

/** This tab's client id, created on first use. */
export function clientId(): string {
  if (cached) return cached
  try {
    const existing = sessionStorage.getItem(STORAGE_KEY)
    if (existing) {
      cached = existing
      return cached
    }
    cached = generate()
    sessionStorage.setItem(STORAGE_KEY, cached)
  } catch {
    // Storage can throw (private mode, disabled cookies). An in-memory id is
    // still correct for the life of the page; it just won't survive a reload,
    // which costs one duplicated event batch rather than breaking anything.
    cached = cached ?? generate()
  }
  return cached
}

/** Reset for tests. */
export function resetClientIdForTests(): void {
  cached = null
  try {
    sessionStorage.removeItem(STORAGE_KEY)
  } catch {
    /* nothing to clear */
  }
}
