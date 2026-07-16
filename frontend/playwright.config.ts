import { defineConfig, devices } from '@playwright/test'
import { existsSync } from 'node:fs'

/**
 * Some sandboxed environments pre-install a specific Chromium build outside
 * Playwright's normal cache and set `PLAYWRIGHT_BROWSERS_PATH` to it, but
 * ship a different revision than this `@playwright/test` version's default
 * "Desktop Chrome" device expects (which launches a headless-shell binary
 * that isn't present). Point `executablePath` at that build directly when
 * it's there; otherwise leave it undefined so Playwright resolves its own
 * browsers normally (e.g. after a real `playwright install`).
 */
const sandboxChromium =
  process.env.PLAYWRIGHT_BROWSERS_PATH && existsSync(`${process.env.PLAYWRIGHT_BROWSERS_PATH}/chromium`)
    ? `${process.env.PLAYWRIGHT_BROWSERS_PATH}/chromium`
    : undefined

/**
 * Playwright configuration for the Outpost 3 frontend (browser/web-host mode).
 *
 * The desktop (Tauri) shell can't be launched headlessly in most CI/sandbox
 * environments, so these tests drive the Vite dev build in a plain browser —
 * `isTauri` is false, so the app falls back to the HTTP/WebSocket client
 * (`outpost_web`). Tests that need live game state must start `outpost_web`
 * separately; pure UI-shell tests (routing, static rendering) need no backend.
 *
 * Uses the environment's pre-installed Chromium
 * (`PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers`) — do not run
 * `playwright install`.
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: 'list',
  use: {
    baseURL: 'http://localhost:4173',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        ...(sandboxChromium ? { launchOptions: { executablePath: sandboxChromium } } : {}),
      },
    },
  ],
  webServer: {
    command: 'npm run build && npm run preview -- --port 4173',
    url: 'http://localhost:4173',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
