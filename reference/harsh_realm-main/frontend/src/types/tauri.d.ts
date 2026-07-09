/**
 * Minimal ambient typings for the global Tauri API.
 *
 * The shell is configured with `withGlobalTauri: true`, which injects
 * `window.__TAURI__` when (and only when) the frontend runs inside the Tauri
 * desktop window. In a browser or the PyWebView build it is absent, so it is
 * declared optional and must be feature-detected before use.
 */

export {}

declare global {
  interface TauriCoreApi {
    invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>
  }

  interface TauriGlobal {
    core: TauriCoreApi
  }

  interface Window {
    __TAURI__?: TauriGlobal
  }
}
