import { defineStore } from "pinia";
import { ref } from "vue";
import type { RuntimeConfig } from "../types/api";

/**
 * Runtime configuration the backend exposes via GET /api/config, so the UI can
 * adapt to admin mode and how the app was launched (desktop vs server).
 */
export const useConfigStore = defineStore("config", () => {
  const adminMode = ref(false);
  const runMode = ref<string>("server");
  const version = ref<string>("");
  const loaded = ref(false);

  async function load(): Promise<void> {
    try {
      const r = await fetch("/api/config");
      if (!r.ok) return;
      const data = (await r.json()) as RuntimeConfig;
      adminMode.value = Boolean(data.admin_mode);
      runMode.value = String(data.run_mode ?? "server");
      version.value = String(data.version ?? "");
      loaded.value = true;
    } catch {
      /* leave defaults */
    }
  }

  return { adminMode, runMode, version, loaded, load };
});
