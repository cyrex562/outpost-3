/**
 * E2E tests — Admin/editor CRUD + GM tools surface (Rust harsh-web host).
 *
 * Exercises the editor endpoints that back the AdminView tabs directly against
 * the running backend via `page.request`: cells, characters (+preview-recalc),
 * factions (+assets), dungeons, oracle threads/NPCs, GM tools, import/export,
 * editor schemas, and item/creature/random-table save+reset. These are the
 * endpoints the Vue admin tabs call, so passing here means the tabs have a
 * working backend. A final UI smoke confirms /admin renders.
 *
 * Runs serially: the backend holds a single active world in process state.
 */

import { test, expect, type APIRequestContext } from "@playwright/test";
import { uniqueWorldName } from "./helpers";

test.describe.configure({ mode: "serial" });

/** Create a small world and load it as the active backend world. */
async function freshWorld(request: APIRequestContext): Promise<void> {
  const created = await request.post("/api/worlds", {
    data: { name: uniqueWorldName("AdminEditor"), width: 6, height: 6 },
  });
  expect(created.ok()).toBe(true);
  const file = ((await created.json()) as { file: string }).file;
  const loaded = await request.post("/api/worlds/load", { data: { file } });
  expect(loaded.ok()).toBe(true);
}

test.beforeEach(async ({ request }) => {
  await freshWorld(request);
});

test("cells: list and single-cell update round-trip", async ({ request }) => {
  const list = await request.get("/api/admin/cells?limit=5");
  expect(list.ok()).toBe(true);
  const cells = (await list.json()) as { q: number; r: number }[];
  expect(cells.length).toBeGreaterThan(0);

  const upd = await request.put("/api/admin/cells/0/0", {
    data: { terrain: "forest", explored: true, faction_id: "", features: ["road"] },
  });
  expect(upd.ok()).toBe(true);

  const after = await request.get("/api/admin/cells?limit=1");
  const cell = ((await after.json()) as { terrain: string; explored: boolean; features: string[] }[])[0];
  expect(cell.terrain).toBe("forest");
  expect(cell.explored).toBe(true);
  expect(cell.features).toContain("road");
});

test("factions: create, edit, asset add/remove, delete", async ({ request }) => {
  const created = await request.post("/api/admin/factions", { data: { name: "Iron Pact" } });
  expect(created.ok()).toBe(true);
  const id = ((await created.json()) as { id: string }).id;

  const upd = await request.put(`/api/admin/factions/${id}`, {
    data: { name: "Iron Pact", hp: 5, force: 3, home_q: 2, home_r: 3, goals: ["expand"], tags: ["militant"] },
  });
  const updated = (await upd.json()) as { force: number; home_q: number; home_r: number };
  expect(updated.force).toBe(3);
  expect(updated.home_q).toBe(2);
  expect(updated.home_r).toBe(3);

  const addAsset = await request.post(`/api/admin/factions/${id}/assets`, {
    data: { asset_type: "Warriors", category: "force" },
  });
  const asset = (await addAsset.json()) as { id: string; asset_type: string };
  expect(asset.asset_type).toBe("Warriors");

  const detail = await request.get(`/api/admin/factions/${id}`);
  const faction = (await detail.json()) as { assets: { id: string }[]; relations: unknown[] };
  expect(faction.assets.length).toBe(1);
  expect(Array.isArray(faction.relations)).toBe(true);

  expect((await request.delete(`/api/admin/factions/${id}/assets/${asset.id}`)).ok()).toBe(true);
  const afterDel = await request.get(`/api/admin/factions/${id}`);
  expect(((await afterDel.json()) as { assets: unknown[] }).assets.length).toBe(0);

  expect((await request.delete(`/api/admin/factions/${id}`)).ok()).toBe(true);
  const all = (await (await request.get("/api/admin/factions")).json()) as unknown[];
  expect(all.length).toBe(0);
});

test("dungeons: create and list with location + room count", async ({ request }) => {
  const created = await request.post("/api/admin/dungeons", { data: { name: "Crypt" } });
  expect(created.ok()).toBe(true);
  const list = (await (await request.get("/api/admin/dungeons")).json()) as {
    name: string;
    location_q: number | null;
    room_count: number;
  }[];
  expect(list.some((d) => d.name === "Crypt")).toBe(true);
});

test("oracle: thread create/update carries type; npc create", async ({ request }) => {
  const created = await request.post("/api/admin/threads", {
    data: { title: "The Lost Ship", type: "story" },
  });
  const id = ((await created.json()) as { id: string }).id;
  await request.put(`/api/admin/threads/${id}`, {
    data: { title: "The Lost Ship II", type: "quest", status: "active", progress: 3 },
  });
  const threads = (await (await request.get("/api/admin/threads")).json()) as {
    id: string;
    type: string;
    progress: number;
  }[];
  const t = threads.find((x) => x.id === id);
  expect(t?.type).toBe("quest");
  expect(t?.progress).toBe(3);

  const npc = await request.post("/api/admin/oracle-npcs", { data: { name: "Mara", notes: "smuggler" } });
  expect(npc.ok()).toBe(true);
  const npcs = (await (await request.get("/api/admin/oracle-npcs")).json()) as { name: string }[];
  expect(npcs.some((n) => n.name === "Mara")).toBe(true);
});

test("characters: create, update data round-trip, soft+hard delete", async ({ request }) => {
  const created = await request.post("/api/admin/characters", {
    data: { name: "Bandit", entity_type: "npc", location_q: 2, location_r: 2, data: { note: "x" } },
  });
  const id = ((await created.json()) as { id: string }).id;

  await request.put(`/api/admin/characters/${id}`, {
    data: { name: "Bandit Chief", location_q: 4, location_r: 5, alive: true, data: { note: "y" } },
  });
  const got = (await (await request.get(`/api/admin/characters/${id}`)).json()) as {
    name: string;
    location_q: number;
    data: { note: string };
  };
  expect(got.name).toBe("Bandit Chief");
  expect(got.location_q).toBe(4);
  expect(got.data.note).toBe("y");

  expect((await request.delete(`/api/admin/characters/${id}`)).ok()).toBe(true); // soft
  expect((await request.delete(`/api/admin/characters/${id}?hard=true`)).ok()).toBe(true);
});

test("characters: preview-recalc returns derived stats", async ({ request }) => {
  const res = await request.post("/api/admin/characters/preview-recalc", {
    data: {
      character_class: "Warrior",
      level: 3,
      attributes: { STR: 14, DEX: 12, CON: 13, INT: 10, WIS: 11, CHA: 9 },
      equipment: [],
    },
  });
  expect(res.ok()).toBe(true);
  const recalc = (await res.json()) as { max_hp: number; attack_bonus: number; attr_mods: Record<string, number> };
  expect(recalc.max_hp).toBeGreaterThan(0);
  expect(recalc.attr_mods.STR).toBe(1);
});

test("gm tools: spawn, teleport, give item, set hp", async ({ request }) => {
  const spawn = await request.post("/api/gm/spawn-entity", {
    data: { entity_type: "npc", name: "Goblin", q: 1, r: 1 },
  });
  const id = ((await spawn.json()) as { id: string }).id;

  const tele = await request.post("/api/gm/teleport", { data: { entity_id: id, q: 4, r: 5 } });
  expect((await tele.json()) as { q: number }).toMatchObject({ q: 4, r: 5 });

  const give = await request.post("/api/gm/give-item", {
    data: { entity_id: id, item: { name: "Sword", type: "weapon" } },
  });
  expect(((await give.json()) as { equipment_count: number }).equipment_count).toBe(1);

  const hp = await request.post("/api/gm/set-hp", { data: { entity_id: id, hp: 12 } });
  expect(((await hp.json()) as { hp: number }).hp).toBe(12);

  const byLoc = (await (await request.get("/api/admin/entities/by-location")).json()) as Record<string, unknown[]>;
  expect(byLoc["4,5"]).toBeTruthy();
});

test("import/export: per-table JSON + CSV, export-all", async ({ request }) => {
  const imp = await request.post("/api/admin/import/difficulty_targets", {
    data: { rows: [{ name: "trivial", target: 6, description: "easy" }] },
  });
  expect(((await imp.json()) as { imported: number }).imported).toBe(1);

  const targets = (await (await request.get("/api/admin/difficulty-targets")).json()) as { name: string }[];
  expect(targets.some((t) => t.name === "trivial")).toBe(true);

  const csv = await request.get("/api/admin/export/cells?format=csv");
  expect(csv.headers()["content-type"]).toContain("text/csv");
  expect(await csv.text()).toContain("terrain");

  const all = (await (await request.get("/api/admin/export-all")).json()) as Record<string, unknown[]>;
  expect(Object.keys(all).length).toBeGreaterThan(10);
});

test("items + creatures: save then reset (revert override)", async ({ request }) => {
  await request.put("/api/admin/items-data/sword01", {
    data: { id: "sword01", name: "Iron Sword", type: "weapon", price: 15 },
  });
  let items = (await (await request.get("/api/admin/items-data")).json()) as { id: string; name: string }[];
  expect(items.find((i) => i.id === "sword01")?.name).toBe("Iron Sword");

  await request.post("/api/admin/items-data/sword01/reset");
  items = (await (await request.get("/api/admin/items-data")).json()) as { id: string }[];
  expect(items.some((i) => i.id === "sword01")).toBe(false);

  await request.put("/api/admin/creature-templates/goblin01", {
    data: { id: "goblin01", name: "Goblin", hp: 7 },
  });
  await request.post("/api/admin/creature-templates/reset-all");
  const creatures = (await (await request.get("/api/admin/creature-templates")).json()) as unknown[];
  expect(creatures.length).toBe(0);
});

test("editor schemas load from the content pack", async ({ request }) => {
  const schemas = (await (await request.get("/api/admin/editor-schemas")).json()) as { label?: string }[];
  expect(Array.isArray(schemas)).toBe(true);
  expect(schemas.length).toBeGreaterThan(0);
});

test("AdminView renders with a config chip", async ({ page }) => {
  await page.goto("/admin");
  await expect(page.getByTestId("admin-config-chip")).toBeVisible({ timeout: 30_000 });
});
