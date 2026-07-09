import { execSync } from "node:child_process";

import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Backend URL is configurable via environment variables so that Docker
// containers can point the proxy at the backend service by name rather than
// localhost. These are Node.js process.env vars, not browser-side VITE_ vars.
const backendHttp =
  process.env.VITE_BACKEND_URL ?? "http://127.0.0.1:8080";
const backendWs =
  process.env.VITE_WS_BACKEND_URL ?? "ws://127.0.0.1:8080";

// Build stamp injected into the bundle at build time (HR-freshness): the git
// short-hash (with a `+` suffix if the tree is dirty) plus the build time. It is
// shown in the app header so a stale desktop/server build is obvious at a glance
// — if the hash doesn't match the deployed commit, the build didn't take.
function buildStamp(): string {
  let hash = process.env.HARSH_BUILD_HASH ?? "unknown";
  try {
    hash = execSync("git rev-parse --short HEAD", {
      stdio: ["ignore", "pipe", "ignore"],
    })
      .toString()
      .trim();
    const dirty =
      execSync("git status --porcelain", {
        stdio: ["ignore", "pipe", "ignore"],
      })
        .toString()
        .trim().length > 0;
    if (dirty) hash += "+";
  } catch {
    // git not available at build time — keep the env/unknown fallback.
  }
  const time = new Date().toISOString().slice(0, 16).replace("T", " ");
  return `${hash} · ${time}Z`;
}

export default defineConfig({
  plugins: [vue()],
  define: {
    __BUILD_STAMP__: JSON.stringify(buildStamp()),
  },
  server: {
    port: 5173,
    host: "0.0.0.0",   // bind to all interfaces so Docker port mapping works
    proxy: {
      "/api": {
        target: backendHttp,
        changeOrigin: true,
      },
      "/ws": {
        target: backendWs,
        ws: true,
        changeOrigin: true,
      },
    },
  },
});
