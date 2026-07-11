/**
 * Unified API layer — transparently uses Tauri IPC when running as a desktop
 * app, or falls back to the HTTP/WebSocket client when running in a browser.
 */

import { tauriApi } from './tauri'
import { httpApi } from './client'

export const api = tauriApi.isTauri ? tauriApi : httpApi
export type { Snapshot } from './client'
