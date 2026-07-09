import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useDifficultyStore } from "./difficulty";
import type { DifficultyDimension } from "../types/api";

function dim(overrides: Partial<DifficultyDimension> = {}): DifficultyDimension {
  return {
    key: "enemy_hp",
    label: "Enemy HP",
    description: "Tougher enemies survive longer.",
    grade: 3,
    grade_count: 7,
    easy_label: "Weakest",
    hard_label: "Toughest",
    normal_grade: 3,
    current_effect: "×1",
    ...overrides,
  };
}

function okResponse(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

function errResponse(status: number): Response {
  return { ok: false, status, json: async () => ({}) } as Response;
}

describe("useDifficultyStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ---- load ---------------------------------------------------------------

  it("load() populates dimensions from GET", async () => {
    const dims = [dim(), dim({ key: "xp", label: "XP Rate", current_effect: "×1" })];
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(okResponse({ dimensions: dims })),
    );
    const store = useDifficultyStore();
    await store.load();
    expect(store.dimensions.length).toBe(2);
    expect(store.dimensions[0].key).toBe("enemy_hp");
    expect(store.dimensions[1].key).toBe("xp");
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it("load() sets error on non-ok response", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(errResponse(500)));
    const store = useDifficultyStore();
    await store.load();
    expect(store.error).toBe("Failed to load difficulty settings (500)");
    expect(store.dimensions).toEqual([]);
  });

  it("load() sets error on network failure", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("boom")));
    const store = useDifficultyStore();
    await store.load();
    expect(store.error).toBe("boom");
    expect(store.loading).toBe(false);
  });

  // ---- setGrade -----------------------------------------------------------

  it("setGrade() optimistically updates then reconciles with the PUT response", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(okResponse({ dimensions: [dim({ grade: 3 })] }))
      .mockResolvedValueOnce(
        okResponse(dim({ grade: 5, current_effect: "×1.5" })),
      );
    vi.stubGlobal("fetch", fetchMock);
    const store = useDifficultyStore();
    await store.load();

    await store.setGrade("enemy_hp", 5);

    // Reconciled to the server's authoritative dimension.
    expect(store.dimensions[0].grade).toBe(5);
    expect(store.dimensions[0].current_effect).toBe("×1.5");
    // PUT body carried the clamped grade.
    const putCall = fetchMock.mock.calls[1];
    expect(putCall[0]).toBe("/api/difficulty/enemy_hp");
    expect(putCall[1].method).toBe("PUT");
    expect(JSON.parse(putCall[1].body)).toEqual({ grade: 5 });
  });

  it("setGrade() reverts on non-ok PUT", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(okResponse({ dimensions: [dim({ grade: 3 })] }))
      .mockResolvedValueOnce(errResponse(400));
    vi.stubGlobal("fetch", fetchMock);
    const store = useDifficultyStore();
    await store.load();

    await store.setGrade("enemy_hp", 6);
    expect(store.dimensions[0].grade).toBe(3); // reverted
    expect(store.error).toBe("Failed to save Enemy HP (400)");
  });

  it("setGrade() reverts on network exception", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(okResponse({ dimensions: [dim({ grade: 3 })] }))
      .mockRejectedValueOnce(new Error("offline"));
    vi.stubGlobal("fetch", fetchMock);
    const store = useDifficultyStore();
    await store.load();

    await store.setGrade("enemy_hp", 1);
    expect(store.dimensions[0].grade).toBe(3); // reverted
    expect(store.error).toBe("offline");
  });

  it("setGrade() clamps grade above range before sending", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(okResponse({ dimensions: [dim({ grade: 3 })] }))
      .mockResolvedValueOnce(okResponse(dim({ grade: 6 })));
    vi.stubGlobal("fetch", fetchMock);
    const store = useDifficultyStore();
    await store.load();

    await store.setGrade("enemy_hp", 99);
    expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toEqual({ grade: 6 });
  });

  it("setGrade() clamps grade below range before sending", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(okResponse({ dimensions: [dim({ grade: 3 })] }))
      .mockResolvedValueOnce(okResponse(dim({ grade: 0 })));
    vi.stubGlobal("fetch", fetchMock);
    const store = useDifficultyStore();
    await store.load();

    await store.setGrade("enemy_hp", -5);
    expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toEqual({ grade: 0 });
  });

  it("setGrade() is a no-op for an unknown key", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(okResponse({ dimensions: [dim({ grade: 3 })] }));
    vi.stubGlobal("fetch", fetchMock);
    const store = useDifficultyStore();
    await store.load();

    await store.setGrade("does_not_exist", 5);
    // No PUT issued (only the initial GET).
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});
