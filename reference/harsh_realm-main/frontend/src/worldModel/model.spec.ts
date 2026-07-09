/**
 * Unit tests for model.ts + reducers.ts — pure TS, no Vue, no jsdom required.
 *
 * The side-effect import of "./reducers" populates `reducerRegistry` so that
 * `apply()` can dispatch to the implemented reducers.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  createWorldModel,
  apply,
  subscribe,
  hydrateWorld,
  type WorldModel,
  type Change,
} from "./model";
import "./reducers"; // populate reducerRegistry
import type {
  ClientEvent,
  CharacterSnapshot,
} from "../types/events.gen";

// ---------------------------------------------------------------------------
// Shared fixture helpers
// ---------------------------------------------------------------------------

/** Minimal CharacterSnapshot satisfying all required fields. */
const stubCharSnapshot: CharacterSnapshot = {
  id: "char-1",
  name: "Hero",
  character_class: "Warrior",
  level: 1,
  xp: 0,
  xp_next: 100,
  attributes: {},
  attr_mods: {},
  skills: {},
  hp: 10,
  max_hp: 10,
  ac: 12,
  attack_bonus: 0,
  physical_save: 15,
  evasion_save: 15,
  mental_save: 15,
  equipment: [],
  class_abilities: {},
  position_q: 0,
  position_r: 0,
};

// ---------------------------------------------------------------------------
// createWorldModel
// ---------------------------------------------------------------------------

describe("createWorldModel", () => {
  it("returns a model with three empty grids and null player", () => {
    const model = createWorldModel();
    expect(model.world.kind).toBe("world");
    expect(model.town.kind).toBe("town");
    expect(model.encounter.kind).toBe("encounter");
    expect(model.world.cells.size).toBe(0);
    expect(model.player).toBeNull();
    expect(model.scene).toBe("exploration");
  });
});

// ---------------------------------------------------------------------------
// subscribe / notify
// ---------------------------------------------------------------------------

describe("subscribe", () => {
  it("calls the listener when apply emits a change", () => {
    const model = createWorldModel();
    const received: Change[] = [];
    subscribe(model, (c) => { received.push(c); });

    const event: ClientEvent = {
      event_type: "town.move",
      data: { direction: "n", player_q: 3, player_r: 4, terrain: "road" },
    };
    apply(model, event);
    expect(received).toHaveLength(1);
    expect(received[0]).toEqual({ kind: "player" });
  });

  it("returns an unsubscribe function that stops future notifications", () => {
    const model = createWorldModel();
    const received: Change[] = [];
    const unsubscribe = subscribe(model, (c) => { received.push(c); });

    unsubscribe();
    apply(model, {
      event_type: "town.move",
      data: { direction: "n", player_q: 0, player_r: 0, terrain: "road" },
    });
    expect(received).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// apply — unhandled event
// ---------------------------------------------------------------------------

describe("apply — unhandled event type", () => {
  it("warns for a genuinely-unknown event type", () => {
    const model = createWorldModel();
    const warnSpy = vi
      .spyOn(console, "warn")
      .mockImplementation((): void => { /* suppress */ });

    // An event type absent from both the reducer registry and the suppressed
    // set — the HR-792 failure class.  Cast because ClientEvent's union only
    // contains known types.
    const event = {
      event_type: "totally.unknown_event",
      data: {},
    } as unknown as ClientEvent;
    expect(() => apply(model, event)).not.toThrow();
    expect(warnSpy).toHaveBeenCalledOnce();

    warnSpy.mockRestore();
  });

  it("does NOT warn for an intentionally-suppressed event (D-6)", () => {
    const model = createWorldModel();
    const warnSpy = vi
      .spyOn(console, "warn")
      .mockImplementation((): void => { /* suppress */ });

    // oracle.fate_check has no reducer but is in SUPPRESSED_GAME_EVENTS — the
    // dev-warn must stay quiet so suppressed events don't spam the console.
    const event: ClientEvent = {
      event_type: "oracle.fate_check",
      data: {
        question: "Will it rain?",
        likelihood: "likely",
        chaos_factor: 5,
        roll: 42,
      },
    };
    expect(() => apply(model, event)).not.toThrow();
    expect(warnSpy).not.toHaveBeenCalled();

    warnSpy.mockRestore();
  });
});

// ---------------------------------------------------------------------------
// exploration.moved
// ---------------------------------------------------------------------------

describe("apply — exploration.moved", () => {
  let model: WorldModel;
  beforeEach(() => { model = createWorldModel(); });

  it("sets the target cell in the world grid", () => {
    const event: ClientEvent = {
      event_type: "exploration.moved",
      data: {
        character_id: "char-1",
        character_data: stubCharSnapshot,
        from_q: 0,
        from_r: 0,
        to_q: 1,
        to_r: 2,
        direction: "se",
        first_visit: true,
        target_cell: { q: 1, r: 2, terrain: "forest", features: [], explored: 1 },
        adjacent_cells: [],
        weather: null,
      },
    };
    apply(model, event);
    const cell = model.world.cells.get("1,2");
    expect(cell?.terrain).toBe("forest");
    expect(cell?.explored).toBe(1);
  });

  it("also inserts each adjacent cell", () => {
    const event: ClientEvent = {
      event_type: "exploration.moved",
      data: {
        character_id: "char-1",
        character_data: stubCharSnapshot,
        from_q: 0,
        from_r: 0,
        to_q: 1,
        to_r: 1,
        direction: "se",
        first_visit: false,
        target_cell: { q: 1, r: 1, terrain: "plains", features: [], explored: 1 },
        adjacent_cells: [
          { q: 0, r: 1, terrain: "forest", features: [], explored: 2 },
          { q: 2, r: 1, terrain: "mountain", features: ["ruins"], explored: 2 },
        ],
        weather: null,
      },
    };
    apply(model, event);
    expect(model.world.cells.size).toBe(3);
    expect(model.world.cells.get("0,1")?.terrain).toBe("forest");
    expect(model.world.cells.get("2,1")?.terrain).toBe("mountain");
    expect(model.world.cells.get("2,1")?.features).toEqual(["ruins"]);
  });

  it("updates player to to_q / to_r", () => {
    apply(model, {
      event_type: "exploration.moved",
      data: {
        character_id: "char-1",
        character_data: stubCharSnapshot,
        from_q: 0,
        from_r: 0,
        to_q: 5,
        to_r: 7,
        direction: "n",
        first_visit: false,
        target_cell: { q: 5, r: 7, terrain: "plains", features: [], explored: 1 },
        adjacent_cells: [],
        weather: null,
      },
    });
    expect(model.player).toEqual({ q: 5, r: 7 });
  });

  // Performance: the world change must be INCREMENTAL — carrying only the
  // target + adjacent cells — so the projection does an O(1)-per-cell Map.set
  // instead of rebuilding the whole cells Map on every step.
  it("emits an incremental world change carrying target + adjacent cells", () => {
    const changes: Change[] = [];
    subscribe(model, (c) => { changes.push(c); });
    apply(model, {
      event_type: "exploration.moved",
      data: {
        character_id: "char-1",
        character_data: stubCharSnapshot,
        from_q: 0,
        from_r: 0,
        to_q: 1,
        to_r: 1,
        direction: "se",
        first_visit: false,
        target_cell: { q: 1, r: 1, terrain: "plains", features: [], explored: 1 },
        adjacent_cells: [
          { q: 0, r: 1, terrain: "forest", features: [], explored: 2 },
        ],
        weather: null,
      },
    });
    const worldChange = changes.find((c) => c.kind === "world");
    expect(worldChange).toBeDefined();
    // Narrow: worldChange is the { kind: "world"; cells?: Cell[] } variant.
    if (worldChange?.kind !== "world") throw new Error("expected world change");
    expect(worldChange.cells).toBeDefined();
    const coords = worldChange.cells?.map((c) => `${c.q},${c.r}`).sort();
    expect(coords).toEqual(["0,1", "1,1"]);
  });

  // D-4: weather must be set from the payload when present, and CLEARED (null)
  // when the entered hex has no weather — otherwise the previous hex's weather
  // lingers on the character.
  it("sets character weather when the payload carries it", () => {
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
    apply(model, {
      event_type: "exploration.moved",
      data: {
        character_id: "char-1",
        character_data: stubCharSnapshot,
        from_q: 0,
        from_r: 0,
        to_q: 1,
        to_r: 0,
        direction: "e",
        first_visit: false,
        target_cell: { q: 1, r: 0, terrain: "plains", features: [], explored: 1 },
        adjacent_cells: [],
        weather: {
          condition: "storm",
          temperature: "cold",
          description: "A driving rainstorm.",
        },
      },
    });
    expect(model.character?.weather).toEqual({
      condition: "storm",
      temperature: "cold",
      description: "A driving rainstorm.",
    });
  });

  it("clears character weather when the entered hex has weather=null", () => {
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
      // Previous hex's weather that must be cleared on the next move.
      weather: {
        condition: "storm",
        temperature: "cold",
        description: "A driving rainstorm.",
      },
      conditions: [],
      statusEffects: [],
      equipment: [],
    };
    apply(model, {
      event_type: "exploration.moved",
      data: {
        character_id: "char-1",
        character_data: stubCharSnapshot,
        from_q: 1,
        from_r: 0,
        to_q: 2,
        to_r: 0,
        direction: "e",
        first_visit: false,
        target_cell: { q: 2, r: 0, terrain: "plains", features: [], explored: 1 },
        adjacent_cells: [],
        weather: null,
      },
    });
    expect(model.character?.weather).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// exploration.revealed
// ---------------------------------------------------------------------------

describe("apply — exploration.revealed", () => {
  it("sets the revealed cell in the world grid with explored=2", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "exploration.revealed",
      data: { cell: { q: 3, r: 4, terrain: "swamp", features: ["fog"], explored: 2 } },
    });
    const cell = model.world.cells.get("3,4");
    expect(cell?.terrain).toBe("swamp");
    expect(cell?.explored).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// travel.step
// ---------------------------------------------------------------------------

describe("apply — travel.step", () => {
  it("sets target cell and updates player position", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "travel.step",
      data: {
        character_id: "char-1",
        from_q: 0,
        from_r: 0,
        to_q: 3,
        to_r: 5,
        direction: "e",
        first_visit: false,
        target_cell: { q: 3, r: 5, terrain: "hills", features: [], explored: 1 },
      },
    });
    expect(model.world.cells.get("3,5")?.terrain).toBe("hills");
    expect(model.player).toEqual({ q: 3, r: 5 });
  });
});

// ---------------------------------------------------------------------------
// town.map
// ---------------------------------------------------------------------------

describe("apply — town.map", () => {
  it("rebuilds the town grid and sets player position", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "town.map",
      data: {
        cells: [
          { q: 0, r: 0, terrain: "road", features: [] },
          { q: 1, r: 0, terrain: "building", features: ["door"] },
        ],
        buildings: [],
        player_q: 0,
        player_r: 0,
        settlement_name: "Dusthaven",
      },
    });
    expect(model.town.cells.size).toBe(2);
    expect(model.town.cells.get("0,0")?.terrain).toBe("road");
    expect(model.town.cells.get("1,0")?.terrain).toBe("building");
    expect(model.player).toEqual({ q: 0, r: 0 });
  });

  it("clears previous town cells before rebuilding", () => {
    const model = createWorldModel();
    // First town.map
    apply(model, {
      event_type: "town.map",
      data: {
        cells: [{ q: 0, r: 0, terrain: "road", features: [] }],
        buildings: [],
        player_q: 0,
        player_r: 0,
        settlement_name: "Old Town",
      },
    });
    // Second town.map (new settlement)
    apply(model, {
      event_type: "town.map",
      data: {
        cells: [
          { q: 5, r: 5, terrain: "plaza", features: [] },
        ],
        buildings: [],
        player_q: 5,
        player_r: 5,
        settlement_name: "New Town",
      },
    });
    // Old cell must be gone
    expect(model.town.cells.get("0,0")).toBeUndefined();
    expect(model.town.cells.get("5,5")?.terrain).toBe("plaza");
    expect(model.player).toEqual({ q: 5, r: 5 });
  });
});

// ---------------------------------------------------------------------------
// town.move
// ---------------------------------------------------------------------------

describe("apply — town.move", () => {
  it("updates player position within the town", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "town.move",
      data: { direction: "n", player_q: 2, player_r: 3, terrain: "road" },
    });
    expect(model.player).toEqual({ q: 2, r: 3 });
  });
});

// ---------------------------------------------------------------------------
// combat.positions
// ---------------------------------------------------------------------------

describe("apply — combat.positions", () => {
  it("rebuilds encounter cells and entities", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "combat.positions",
      data: {
        width: 9,
        height: 9,
        grid_type: "square",
        cells: [
          { q: 0, r: 0, terrain: "dirt", features: [] },
          { q: 4, r: 4, terrain: "dirt", features: [] },
        ],
        entities: [
          {
            entity_id: "pc-1",
            kind: "pc",
            name: "Hero",
            q: 4,
            r: 4,
            hp: 10,
            max_hp: 10,
            alive: true,
          },
          {
            entity_id: "goblin-1",
            kind: "monster",
            name: "Goblin",
            q: 2,
            r: 2,
            hp: 5,
            max_hp: 5,
            alive: true,
          },
        ],
        loot: [],
      },
    });
    expect(model.encounter.width).toBe(9);
    expect(model.encounter.height).toBe(9);
    expect(model.encounter.cells.size).toBe(2);
    expect(model.encounter.entities).toHaveLength(2);
    const goblin = model.encounter.entities.find((e) => e.id === "goblin-1");
    expect(goblin?.kind).toBe("monster");
    expect(goblin?.hp).toBe(5);
    expect(goblin?.maxHp).toBe(5);
    expect(goblin?.q).toBe(2);
    expect(goblin?.r).toBe(2);
    expect(goblin?.alive).toBe(true);
  });

  it("maps dropped loot into model.combat.loot (HR-787)", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "combat.positions",
      data: {
        width: 9,
        height: 9,
        grid_type: "square",
        cells: [{ q: 4, r: 4, terrain: "dirt", features: [] }],
        entities: [],
        loot: [
          { q: 2, r: 3, value: 150, item_count: 2 },
          { q: 5, r: 6, value: 12, item_count: 1 },
        ],
      },
    });
    expect(model.combat.loot).toEqual([
      { q: 2, r: 3, value: 150, itemCount: 2 },
      { q: 5, r: 6, value: 12, itemCount: 1 },
    ]);
  });

  it("clears loot when leaving combat via gm.scene_change", () => {
    const model = createWorldModel();
    model.combat.loot = [{ q: 1, r: 1, value: 40, itemCount: 1 }];
    apply(model, { event_type: "gm.scene_change", data: { to: "exploration" } });
    expect(model.combat.loot).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// combat.start
// ---------------------------------------------------------------------------

describe("apply — combat.start", () => {
  it("populates encounter entities from enemy_states", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "combat.start",
      data: {
        awareness: "normal",
        enemies: ["Goblin", "Orc"],
        enemy_states: [
          { entity_id: "g-1", name: "Goblin", hp: 5, max_hp: 5 },
          { entity_id: "o-1", name: "Orc", hp: 12, max_hp: 12 },
        ],
      },
    });
    expect(model.encounter.entities).toHaveLength(2);
    const orc = model.encounter.entities.find((e) => e.id === "o-1");
    expect(orc?.name).toBe("Orc");
    expect(orc?.hp).toBe(12);
    expect(orc?.maxHp).toBe(12);
    expect(orc?.alive).toBe(true);
    expect(orc?.kind).toBe("monster");
  });
});

// ---------------------------------------------------------------------------
// combat.enemy_defeated
// ---------------------------------------------------------------------------

describe("apply — combat.enemy_defeated", () => {
  it("marks the entity alive=false", () => {
    const model = createWorldModel();
    // Seed encounter entities via combat.start
    apply(model, {
      event_type: "combat.start",
      data: {
        awareness: "normal",
        enemies: ["Goblin"],
        enemy_states: [{ entity_id: "g-1", name: "Goblin", hp: 5, max_hp: 5 }],
      },
    });
    apply(model, {
      event_type: "combat.enemy_defeated",
      data: { entity_id: "g-1", name: "Goblin" },
    });
    const goblin = model.encounter.entities.find((e) => e.id === "g-1");
    expect(goblin?.alive).toBe(false);
  });

  it("is a no-op for an unknown entity_id", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "combat.enemy_defeated",
      data: { entity_id: "does-not-exist", name: "Ghost" },
    });
    expect(model.encounter.entities).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// gm.scene_change
// ---------------------------------------------------------------------------

describe("apply — gm.scene_change", () => {
  it("sets model.scene", () => {
    const model = createWorldModel();
    apply(model, { event_type: "gm.scene_change", data: { to: "combat" } });
    expect(model.scene).toBe("combat");
  });

  it("clears encounter when leaving combat", () => {
    const model = createWorldModel();
    // Put something in the encounter grid first
    apply(model, {
      event_type: "combat.start",
      data: {
        awareness: "normal",
        enemies: ["Goblin"],
        enemy_states: [{ entity_id: "g-1", name: "Goblin", hp: 5, max_hp: 5 }],
      },
    });
    expect(model.encounter.entities).toHaveLength(1);

    apply(model, { event_type: "gm.scene_change", data: { to: "exploration" } });
    expect(model.encounter.entities).toHaveLength(0);
    expect(model.encounter.cells.size).toBe(0);
  });

  it("preserves encounter when entering combat scene", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "combat.start",
      data: {
        awareness: "normal",
        enemies: ["Goblin"],
        enemy_states: [{ entity_id: "g-1", name: "Goblin", hp: 5, max_hp: 5 }],
      },
    });
    apply(model, { event_type: "gm.scene_change", data: { to: "combat" } });
    // Encounter was NOT cleared because to === "combat"
    expect(model.encounter.entities).toHaveLength(1);
  });

  it("clears town only when truly leaving (not for shopping/social)", () => {
    const model = createWorldModel();
    // Populate town
    apply(model, {
      event_type: "town.map",
      data: {
        cells: [{ q: 0, r: 0, terrain: "road", features: [] }],
        buildings: [],
        player_q: 0,
        player_r: 0,
        settlement_name: "Test",
      },
    });
    expect(model.town.cells.size).toBe(1);

    // Entering a shop — town must be preserved
    apply(model, { event_type: "gm.scene_change", data: { to: "shopping" } });
    expect(model.town.cells.size).toBe(1);

    // Returning to social — still preserved
    apply(model, { event_type: "gm.scene_change", data: { to: "social" } });
    expect(model.town.cells.size).toBe(1);

    // Leaving to exploration — town must be cleared
    apply(model, { event_type: "gm.scene_change", data: { to: "exploration" } });
    expect(model.town.cells.size).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// hydrateWorld
// ---------------------------------------------------------------------------

describe("hydrateWorld", () => {
  it("loads width, height, and cells into the world grid", () => {
    const model = createWorldModel();
    hydrateWorld(model, {
      width: 32,
      height: 32,
      cells: [
        { q: 0, r: 0, terrain: "plains", features: [], explored: 1 },
        { q: 1, r: 0, terrain: "forest", features: ["ancient_tree"], explored: 1 },
        { q: 0, r: 1, terrain: "plains", features: [], explored: 0 },
      ],
    });
    expect(model.world.width).toBe(32);
    expect(model.world.height).toBe(32);
    expect(model.world.cells.size).toBe(3);
    expect(model.world.cells.get("1,0")?.terrain).toBe("forest");
    expect(model.world.cells.get("1,0")?.features).toEqual(["ancient_tree"]);
    expect(model.world.cells.get("0,1")?.explored).toBe(0);
  });

  it("replaces existing world cells on a second call", () => {
    const model = createWorldModel();
    hydrateWorld(model, {
      width: 10,
      height: 10,
      cells: [{ q: 0, r: 0, terrain: "old", features: [], explored: 1 }],
    });
    hydrateWorld(model, {
      width: 20,
      height: 20,
      cells: [{ q: 5, r: 5, terrain: "new", features: [], explored: 1 }],
    });
    expect(model.world.width).toBe(20);
    expect(model.world.cells.get("0,0")).toBeUndefined();
    expect(model.world.cells.get("5,5")?.terrain).toBe("new");
  });

  it("notifies subscribers with a BARE { kind: 'world' } (full rebuild, no cells)", () => {
    const model = createWorldModel();
    const changes: Change[] = [];
    subscribe(model, (c) => { changes.push(c); });
    hydrateWorld(model, { width: 1, height: 1, cells: [] });
    // Bare change (no `cells`) tells the projection to do a full rebuild.
    expect(changes).toEqual([{ kind: "world" }]);
    const worldChange = changes[0];
    if (worldChange?.kind !== "world") throw new Error("expected world change");
    expect(worldChange.cells).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// loot.source_revealed (HR-786)
// ---------------------------------------------------------------------------

describe("apply — loot.source_revealed", () => {
  function reveal(
    q: number,
    r: number,
    id: string,
    items: Array<Record<string, unknown>>,
    gold: number,
  ) {
    return {
      event_type: "loot.source_revealed" as const,
      data: { q, r, id, kind: "container", name: "crate", items, gold },
    };
  }

  it("adds a revealed source to model.loot", () => {
    const model = createWorldModel();
    apply(model, reveal(1, 2, "crate_1", [{ name: "Rope", value: 5 }], 10));
    expect(model.loot).toHaveLength(1);
    expect(model.loot[0].id).toBe("crate_1");
    expect(model.loot[0].gold).toBe(10);
  });

  it("replaces an existing source's contents on re-emit (after a take)", () => {
    const model = createWorldModel();
    apply(model, reveal(1, 2, "crate_1", [{ name: "Rope" }, { name: "Ruby" }], 10));
    apply(model, reveal(1, 2, "crate_1", [{ name: "Ruby" }], 10));
    expect(model.loot).toHaveLength(1);
    expect(model.loot[0].items).toHaveLength(1);
  });

  it("drops an exhausted source (empty items + no gold)", () => {
    const model = createWorldModel();
    apply(model, reveal(1, 2, "crate_1", [{ name: "Rope" }], 0));
    apply(model, reveal(1, 2, "crate_1", [], 0));
    expect(model.loot).toEqual([]);
  });

  it("is cleared when the player moves to a new cell", () => {
    const model = createWorldModel();
    apply(model, reveal(1, 2, "crate_1", [{ name: "Rope" }], 0));
    expect(model.loot).toHaveLength(1);
    apply(model, {
      event_type: "exploration.moved",
      data: {
        character_id: "c",
        character_data: stubCharSnapshot,
        from_q: 1,
        from_r: 2,
        to_q: 2,
        to_r: 2,
        direction: "e",
        first_visit: false,
        target_cell: { q: 2, r: 2, terrain: "plains", features: [], explored: 1 },
        adjacent_cells: [],
        weather: null,
      },
    });
    expect(model.loot).toEqual([]);
  });

  it("is cleared on a scene change", () => {
    const model = createWorldModel();
    apply(model, reveal(1, 2, "crate_1", [{ name: "Rope" }], 0));
    apply(model, { event_type: "gm.scene_change", data: { to: "combat" } });
    expect(model.loot).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// quest.* (HR-19)
// ---------------------------------------------------------------------------

describe("apply — quest.* events", () => {
  it("quest.accepted emits a message and a refetch-quests effect", () => {
    const model = createWorldModel();
    const changes: Change[] = [];
    subscribe(model, (c) => changes.push(c));
    apply(model, {
      event_type: "quest.accepted",
      data: { quest_id: "a", quest_name: "The A", description: "d" },
    });
    expect(changes).toContainEqual({ kind: "refetch-quests" });
    expect(
      changes.some((c) => c.kind === "message" && c.text.includes("The A")),
    ).toBe(true);
  });

  it("quest.progress_updated emits only a refetch-quests effect", () => {
    const model = createWorldModel();
    const changes: Change[] = [];
    subscribe(model, (c) => changes.push(c));
    apply(model, {
      event_type: "quest.progress_updated",
      data: { quest_id: "a", objective_key: "k", value: 2, progress: { k: 2 } },
    });
    expect(changes).toEqual([{ kind: "refetch-quests" }]);
  });

  it("quest.completed emits a message + refetch", () => {
    const model = createWorldModel();
    const changes: Change[] = [];
    subscribe(model, (c) => changes.push(c));
    apply(model, {
      event_type: "quest.completed",
      data: { quest_id: "a", quest_name: "Done", reward_gold: 1, reward_xp: 2, reward_items: [] },
    });
    expect(changes).toContainEqual({ kind: "refetch-quests" });
    expect(changes.some((c) => c.kind === "message" && c.text.includes("Done"))).toBe(true);
  });

  it("quest.failed emits a message + refetch", () => {
    const model = createWorldModel();
    const changes: Change[] = [];
    subscribe(model, (c) => changes.push(c));
    apply(model, {
      event_type: "quest.failed",
      data: { quest_id: "a", quest_name: "Lost" },
    });
    expect(changes).toContainEqual({ kind: "refetch-quests" });
    expect(changes.some((c) => c.kind === "message" && c.text.includes("Lost"))).toBe(true);
  });
});
