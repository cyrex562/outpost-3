import { defineStore } from "pinia";
import { ref } from "vue";

export interface PanelState {
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
  minimized: boolean;
  visible: boolean;
}

const DEFAULT_PANELS: Record<string, PanelState> = {
  chat: {
    x: 0,
    y: 464,
    width: 770,
    height: 360,
    zIndex: 10,
    minimized: false,
    visible: true,
  },
  map: {
    x: 0,
    y: 44,
    width: 530,
    height: 410,
    zIndex: 11,
    minimized: false,
    visible: true,
  },
  status: {
    x: 540,
    y: 44,
    width: 230,
    height: 410,
    zIndex: 12,
    minimized: false,
    visible: true,
  },
  ref: {
    x: 200,
    y: 100,
    width: 380,
    height: 540,
    zIndex: 13,
    minimized: false,
    visible: false,
  },
  inv: {
    x: 780,
    y: 44,
    width: 260,
    height: 410,
    zIndex: 14,
    minimized: false,
    visible: false,
  },
  townmap: {
    x: 10,
    y: 54,
    width: 350,
    height: 350,
    zIndex: 20,
    minimized: false,
    visible: true,
  },
  // Encounter battle-grid panel (HR-755) — auto-shown when entering combat
  encounter: {
    x: 10,
    y: 54,
    width: 370,
    height: 370,
    zIndex: 21,
    minimized: false,
    visible: false,
  },
  // Loot panel (HR-786) — auto-shown when a searchable source is revealed
  loot: {
    x: 400,
    y: 90,
    width: 300,
    height: 320,
    zIndex: 22,
    minimized: false,
    visible: false,
  },
};

export const useLayoutStore = defineStore("layout", () => {
  const panels = ref<Record<string, PanelState>>(
    JSON.parse(JSON.stringify(DEFAULT_PANELS))
  );
  const topZ = ref(50);

  function updatePanel(id: string, partial: Partial<PanelState>): void {
    if (!panels.value[id]) {
      panels.value[id] = { ...DEFAULT_PANELS[id] ?? { x: 100, y: 100, width: 400, height: 300, zIndex: 10, minimized: false, visible: true } };
    }
    panels.value[id] = { ...panels.value[id], ...partial };
  }

  function bringToFront(id: string): void {
    topZ.value += 1;
    updatePanel(id, { zIndex: topZ.value });
  }

  function resetLayout(): void {
    panels.value = JSON.parse(JSON.stringify(DEFAULT_PANELS));
    topZ.value = 50;
  }

  async function saveLayout(): Promise<void> {
    try {
      await fetch("/api/ui/layout", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(panels.value),
      });
    } catch {
      // fire and forget — silently ignore errors
    }
  }

  async function loadLayout(): Promise<void> {
    try {
      const r = await fetch("/api/ui/layout");
      if (!r.ok) return;
      const data = await r.json() as Record<string, PanelState>;
      // Merge: only update panels that exist in the response and current state
      for (const [id, state] of Object.entries(data)) {
        if (panels.value[id]) {
          panels.value[id] = { ...panels.value[id], ...state };
        }
      }
      // Update topZ to be above whatever we loaded
      const maxZ = Math.max(...Object.values(panels.value).map((p) => p.zIndex));
      if (maxZ > topZ.value) {
        topZ.value = maxZ;
      }
    } catch {
      // silently ignore — use defaults
    }
  }

  return {
    panels,
    topZ,
    updatePanel,
    bringToFront,
    resetLayout,
    saveLayout,
    loadLayout,
  };
});
