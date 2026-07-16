import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  test: {
    environment: 'jsdom',
    globals: true,
    // Playwright e2e specs live in e2e/ and use @playwright/test's own
    // test()/expect() — exclude them so vitest doesn't try to run them.
    exclude: ['**/node_modules/**', '**/dist/**', 'e2e/**'],
  },
})
