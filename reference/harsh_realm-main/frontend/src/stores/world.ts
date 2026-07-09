import { defineStore } from "pinia";
import { ref } from "vue";
import { useGameStore } from "./game";
import { useLayoutStore } from "./layout";
import { useMapStore } from "./map";

export interface WorldSummary {
  name: string;
  file: string;
  last_modified: string;
}

export const useWorldStore = defineStore("world", () => {
  const activeWorld = ref<string | null>(null); // active world filename
  const activeName = ref<string | null>(null);
  const available = ref<WorldSummary[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchWorlds() {
    try {
      const r = await fetch("/api/worlds");
      available.value = await r.json();
    } catch {
      // silently ignore — list just stays empty
    }
  }

  /** Populate the active world from the backend (e.g. when deep-linking to a view). */
  async function fetchCurrent(): Promise<void> {
    try {
      const r = await fetch("/api/worlds/current");
      if (!r.ok) return;
      const data = (await r.json()) as { file?: string; name?: string };
      if (data?.file) {
        activeWorld.value = data.file;
        activeName.value = data.name ?? data.file;
      }
    } catch {
      // silently ignore
    }
  }

  async function createWorld(
    name: string,
    width: number,
    height: number,
    seed?: number,
    packIds?: string[],
  ): Promise<boolean> {
    loading.value = true;
    error.value = null;
    try {
      const body: Record<string, unknown> = { name, width, height };
      if (seed !== undefined) body.seed = seed;
      if (packIds !== undefined) body.pack_ids = packIds;
      const r = await fetch("/api/worlds", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (!r.ok) {
        const data = await r.json().catch(() => ({}));
        error.value = data?.detail ?? `Error ${r.status}`;
        return false;
      }
      const data = await r.json();
      return await loadWorld(data.file);
    } catch (e) {
      error.value = String(e);
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function loadWorld(file: string): Promise<boolean> {
    loading.value = true;
    error.value = null;
    // Clear stale game state from previous world
    const gameStore = useGameStore();
    const mapStore = useMapStore();
    gameStore.reset();
    mapStore.reset();
    try {
      const r = await fetch("/api/worlds/load", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file }),
      });
      if (!r.ok) {
        const data = await r.json().catch(() => ({}));
        error.value = data?.detail ?? `Error ${r.status}`;
        return false;
      }
      const data = await r.json();
      activeWorld.value = file;
      activeName.value = data.name ?? file;

      // Force WebSocket reconnection to trigger initial scene narration from GM
      const { connect, disconnect } = (
        await import("../composables/useWebSocket")
      ).useWebSocket();
      disconnect();
      connect();

      // Explicitly load map and character
      await mapStore.loadMap();
      gameStore.loadCharacter();
      // Restore saved panel layout for this world
      useLayoutStore().loadLayout();
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    } finally {
      loading.value = false;
    }
  }

  return {
    activeWorld,
    activeName,
    available,
    loading,
    error,
    fetchWorlds,
    fetchCurrent,
    createWorld,
    loadWorld,
  };
});
