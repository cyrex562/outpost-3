import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useLootStore } from "./loot";
import type { LootSourceView } from "../worldModel/model";

function source(overrides: Partial<LootSourceView> = {}): LootSourceView {
  return {
    q: 1,
    r: 2,
    id: "crate_1",
    kind: "container",
    name: "weathered crate",
    items: [{ name: "Rope", value: 5 }],
    gold: 10,
    ...overrides,
  };
}

describe("useLootStore", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("setSources replaces the current source list", () => {
    const store = useLootStore();
    store.setSources([source(), source({ id: "corpse_1", kind: "corpse" })]);
    expect(store.sources.map((s) => s.id)).toEqual(["crate_1", "corpse_1"]);
  });

  it("clear empties the sources", () => {
    const store = useLootStore();
    store.setSources([source()]);
    store.clear();
    expect(store.sources).toEqual([]);
  });
});
