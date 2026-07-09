/**
 * Vitest configuration for the worldModel unit-test suite (HR-795, 104a).
 *
 * The worldModel is pure TypeScript with zero Vue/DOM dependencies, so a Node
 * environment is both sufficient and fastest.  A separate config (rather than
 * extending vite.config.ts) avoids pulling in the Vite plugins and the
 * buildStamp() git-shell call at test time.
 */
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.spec.ts"],
  },
});
