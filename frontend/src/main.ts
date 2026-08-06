/// <reference types="vite/client" />
import './assets/theme.css'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { logFrontendError } from '@/services/tauriBridge'

const app = createApp(App)
app.use(createPinia())
app.use(router)

// No error boundary exists elsewhere in the tree, so an uncaught exception
// during a component's render/setup/watcher previously vanished — Vue logs
// it to the devtools console and moves on, which from the player's side
// looks like a silently blank panel or screen. Catching it here logs it to
// the backend's persistent log file (see tauriBridge.logFrontendError) so a
// bug report has something to point at even when nobody was watching
// devtools at the time.
app.config.errorHandler = (err, instance, info) => {
  const message = err instanceof Error ? err.message : String(err)
  const stack = err instanceof Error ? err.stack : undefined
  const componentName = instance?.$options?.name ?? instance?.$options?.__name ?? 'unknown'
  console.error(`[vue error] ${info} in <${componentName}>:`, err)
  void logFrontendError('vue-error-handler', `${info} in <${componentName}>: ${message}`, stack)

  // Installing a custom errorHandler suppresses Vue's default dev-mode
  // rethrow, which is what normally makes render/setup errors surface
  // loudly via Vite's overlay. Re-throw here (async, so it doesn't disrupt
  // Vue's own error-handling flow) to keep that loud dev-mode signal —
  // logging should make bug reports diagnosable in production, not make
  // this class of bug quieter for the person writing the fix.
  if (import.meta.env.DEV) {
    setTimeout(() => {
      throw err
    })
  }
}

window.addEventListener('error', (event) => {
  console.error('[window error]', event.error ?? event.message)
  void logFrontendError(
    'window-onerror',
    event.error instanceof Error ? event.error.message : String(event.message),
    event.error instanceof Error ? event.error.stack : undefined,
  )
})

window.addEventListener('unhandledrejection', (event) => {
  const reason = event.reason
  console.error('[unhandled rejection]', reason)
  void logFrontendError(
    'unhandled-rejection',
    reason instanceof Error ? reason.message : String(reason),
    reason instanceof Error ? reason.stack : undefined,
  )
})

app.mount('#app')
