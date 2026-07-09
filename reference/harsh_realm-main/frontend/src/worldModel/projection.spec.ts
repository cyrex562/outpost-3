/**
 * Projection-level tests (HR-795, 104c divergence fixes).
 *
 * The projection maps model Change notifications onto the Pinia stores.  It has
 * no Vue *runtime* imports (store types are `import type`), so it can be driven
 * in the node vitest env with lightweight fake stores that implement only the
 * methods/props the projection touches.
 *
 * Covered here:
 *   D-1 — combat.enemy_defeated clears the selected attack target when the
 *         defeated enemy was the current selection.
 *   D-2 — refetch-character coalesces a burst into a single loadCharacter call.
 */
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { createWorldModel, apply, type WorldModel } from "./model";
import "./reducers";
import { installProjection, type ProjectionDeps } from "./projection";

// ---------------------------------------------------------------------------
// Fake stores — plain objects implementing only what the projection calls.
// ---------------------------------------------------------------------------

interface FakeEncounterStore {
  selectedTargetId: string | null;
  selectTarget: (id: string | null) => void;
  setGrid: (data: unknown) => void;
  clear: () => void;
  lastGrid: unknown;
}

function makeFakeDeps(): {
  deps: ProjectionDeps;
  encounter: FakeEncounterStore;
  loot: { lastSources: unknown };
  gameCharacterChecked: () => boolean;
} {
  const encounter: FakeEncounterStore = {
    selectedTargetId: null,
    lastGrid: null,
    selectTarget(id: string | null): void {
      this.selectedTargetId = id;
    },
    setGrid(data: unknown): void {
      this.lastGrid = data;
    },
    clear(): void {
      this.selectedTargetId = null;
    },
  };

  let characterChecked = false;
  const gameStore = {
    character: null as unknown,
    combatEnemies: [] as unknown[],
    combatActions: [] as unknown[],
    combatPhase: "",
    get characterChecked(): boolean { return characterChecked; },
    set characterChecked(v: boolean) { characterChecked = v; },
    addMessage: (): void => { /* no-op */ },
    setScene: (): void => { /* no-op */ },
    setChaos: (): void => { /* no-op */ },
    addSuggestions: (): void => { /* no-op */ },
    setExplorationActions: (): void => { /* no-op */ },
    setPendingTravel: (): void => { /* no-op */ },
    setAttributeRoll: (): void => { /* no-op */ },
  };

  const mapStore = {
    cells: new Map<string, unknown>(),
    width: 0,
    height: 0,
    loaded: false,
    setPlayerPosition: (): void => { /* no-op */ },
  };

  const townStore = {
    cells: [] as unknown[],
    buildings: [] as unknown[],
    playerQ: 0,
    playerR: 0,
    settlementName: "",
    active: false,
    clear: (): void => { /* no-op */ },
  };

  const loot = {
    lastSources: null as unknown,
    setSources(next: unknown): void {
      this.lastSources = next;
    },
    clear(): void {
      this.lastSources = [];
    },
  };

  const deps = {
    gameStore,
    mapStore,
    townStore,
    encounterStore: encounter,
    lootStore: loot,
  } as unknown as ProjectionDeps;

  return { deps, encounter, loot, gameCharacterChecked: () => characterChecked };
}

// ---------------------------------------------------------------------------

describe("projection — D-1: selected target cleared when its enemy dies", () => {
  let model: WorldModel;
  let uninstall: () => void;
  let encounter: FakeEncounterStore;

  beforeEach(() => {
    model = createWorldModel();
    // Seed a live encounter with two enemies.
    model.encounter.width = 5;
    model.encounter.height = 5;
    model.encounter.entities = [
      { id: "enemy-1", kind: "monster", q: 1, r: 1, name: "Goblin", hp: 5, maxHp: 5, alive: true },
      { id: "enemy-2", kind: "monster", q: 2, r: 2, name: "Orc", hp: 8, maxHp: 8, alive: true },
    ];
    const fake = makeFakeDeps();
    encounter = fake.encounter;
    uninstall = installProjection(model, fake.deps);
  });

  afterEach(() => {
    uninstall();
  });

  it("clears selectedTargetId when the selected enemy is defeated", () => {
    encounter.selectedTargetId = "enemy-1";
    apply(model, {
      event_type: "combat.enemy_defeated",
      data: { entity_id: "enemy-1", name: "Goblin" },
    });
    expect(encounter.selectedTargetId).toBeNull();
  });

  it("keeps the selection when a DIFFERENT enemy is defeated", () => {
    encounter.selectedTargetId = "enemy-1";
    apply(model, {
      event_type: "combat.enemy_defeated",
      data: { entity_id: "enemy-2", name: "Orc" },
    });
    expect(encounter.selectedTargetId).toBe("enemy-1");
  });
});

describe("projection — HR-787: encounter loot reaches the store", () => {
  let model: WorldModel;
  let uninstall: () => void;
  let encounter: FakeEncounterStore;

  beforeEach(() => {
    model = createWorldModel();
    const fake = makeFakeDeps();
    encounter = fake.encounter;
    uninstall = installProjection(model, fake.deps);
  });
  afterEach(() => uninstall());

  it("passes dropped loot piles into encounterStore.setGrid", () => {
    apply(model, {
      event_type: "combat.positions",
      data: {
        width: 9,
        height: 9,
        grid_type: "square",
        cells: [{ q: 4, r: 4, terrain: "dirt", features: [] }],
        entities: [
          {
            entity_id: "goblin-1",
            kind: "monster",
            name: "Goblin",
            q: 2,
            r: 2,
            hp: 0,
            max_hp: 5,
            alive: false,
          },
        ],
        loot: [{ q: 2, r: 2, value: 150, item_count: 2 }],
      },
    });
    const grid = encounter.lastGrid as {
      loot: Array<{ q: number; r: number; value: number; item_count: number }>;
    };
    expect(grid.loot).toEqual([{ q: 2, r: 2, value: 150, item_count: 2 }]);
  });
});

describe("projection — D-2: refetch-character debounce", () => {
  let model: WorldModel;
  let uninstall: () => void;
  let fetchSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.useFakeTimers();
    model = createWorldModel();
    model.character = {
      id: "char-1",
      name: "Hero",
      characterClass: "Warrior",
      level: 1,
      hp: 10,
      maxHp: 10,
      ac: 12,
      xp: 0,
      xpNext: 100,
      gold: 0,
      str: 10,
      locationQ: 0,
      locationR: 0,
      terrain: "plains",
      features: [],
      weather: null,
      conditions: [],
      statusEffects: [],
      equipment: [],
    };
    // Stub fetch so loadCharacter resolves without a real network call.
    fetchSpy = vi.fn(async () => ({
      ok: false,
      json: async () => ({}),
    }));
    vi.stubGlobal("fetch", fetchSpy);
    const fake = makeFakeDeps();
    uninstall = installProjection(model, fake.deps);
  });

  afterEach(() => {
    uninstall();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("coalesces a burst of refetch-character effects into one loadCharacter fetch", () => {
    // Simulate a 3-item loot burst: each inventory.item_lost emits refetch-character.
    for (let i = 0; i < 3; i++) {
      apply(model, {
        event_type: "inventory.item_lost",
        data: { character_id: "char-1", item_name: `item-${i}` },
      });
    }
    // Before the debounce window elapses, no fetch has fired.
    expect(fetchSpy).not.toHaveBeenCalled();
    // Advance past the 60ms trailing debounce → exactly one fetch.
    vi.advanceTimersByTime(60);
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith("/api/character");
  });
});

describe("projection — HR-786: revealed loot reaches the loot store", () => {
  let model: WorldModel;
  let uninstall: () => void;
  let loot: { lastSources: unknown };

  beforeEach(() => {
    model = createWorldModel();
    const fake = makeFakeDeps();
    loot = fake.loot;
    uninstall = installProjection(model, fake.deps);
  });
  afterEach(() => uninstall());

  it("projects model.loot into lootStore.setSources on a revealed source", () => {
    apply(model, {
      event_type: "loot.source_revealed",
      data: {
        q: 1,
        r: 2,
        id: "crate_1",
        kind: "container",
        name: "crate",
        items: [{ name: "Rope", value: 5 }],
        gold: 10,
      },
    });
    const sources = loot.lastSources as Array<{ id: string; gold: number }>;
    expect(sources).toHaveLength(1);
    expect(sources[0].id).toBe("crate_1");
    expect(sources[0].gold).toBe(10);
  });
});
