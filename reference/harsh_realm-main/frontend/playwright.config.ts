import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E configuration for Harsh Realm.
 *
 * Expects:
 *   - Backend:  harsh-web (Rust Axum host)     running on  http://127.0.0.1:8080
 *   - Frontend: vite dev server               running on  http://localhost:5173
 *
 * webServer entries start both servers automatically when they are not already
 * running.  Set REUSE_SERVERS=1 to always reuse existing processes (handy
 * during iterative dev).
 */

const reuseExisting =
  process.env.REUSE_SERVERS === "1" || !process.env.CI;
const skipWebServer = process.env.PLAYWRIGHT_SKIP_WEBSERVER === "1";
const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:5173";
const backendURL =
  process.env.PLAYWRIGHT_BACKEND_URL ?? "http://127.0.0.1:8080/api/worlds";
const reporterOutputFolder =
  process.env.PLAYWRIGHT_REPORT_DIR ?? "playwright-report";

export default defineConfig({
  testDir: "./e2e",

  /** Global per-test timeout. Character creation has several round-trips. */
  timeout: 90_000,

  expect: { timeout: 20_000 },

  /** Run tests in the same file sequentially; files can run in parallel. */
  fullyParallel: false,

  /** Retry once on CI to smooth over transient flakiness. */
  retries: process.env.CI ? 1 : 0,

  /** Workers: 1 so tests share the same backend world state sensibly. */
  workers: 1,

  reporter: [
    ["list"],
    ["html", { open: "never", outputFolder: reporterOutputFolder }],
  ],

  use: {
    baseURL,
    trace: "on-first-retry",
    video: "on-first-retry",
    screenshot: "only-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  webServer: skipWebServer
    ? undefined
    : [
        {
          /** Backend (Rust Axum host) — run from the repo root (one dir up). */
          command:
            "cargo run --quiet --manifest-path crates/harsh-web/Cargo.toml",
          url: backendURL,
          reuseExistingServer: reuseExisting,
          cwd: "..",
          timeout: 180_000,
        },
        {
          /** Vite dev server — run from the frontend/ directory (config file dir). */
          command: "npm run dev",
          url: baseURL,
          reuseExistingServer: reuseExisting,
          timeout: 30_000,
        },
      ],
});
