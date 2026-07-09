import { defineStore } from "pinia";
import { ref, type Ref } from "vue";
import type {
  DifficultyDimension,
  DifficultySettingsResponse,
} from "../types/api";

/**
 * Player-facing adjustable difficulty (HR-107).
 *
 * Mirrors the per-world `difficulty_settings` table via `/api/difficulty`.
 * The dimension list is rendered generically by `DifficultyView.vue`, so this
 * store never hardcodes dimension keys.
 */
export interface DifficultyStore {
  dimensions: Ref<DifficultyDimension[]>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
  load: () => Promise<void>;
  setGrade: (key: string, grade: number) => Promise<void>;
}

export const useDifficultyStore = defineStore("difficulty", (): DifficultyStore => {
  const dimensions = ref<DifficultyDimension[]>([]);
  const loading = ref<boolean>(false);
  const error = ref<string | null>(null);

  /** Fetch all dimensions with their current grades. */
  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const res = await fetch("/api/difficulty");
      if (!res.ok) {
        error.value = `Failed to load difficulty settings (${res.status})`;
        return;
      }
      const body = (await res.json()) as DifficultySettingsResponse;
      dimensions.value = body.dimensions;
    } catch (e) {
      error.value = e instanceof Error ? e.message : "Network error";
    } finally {
      loading.value = false;
    }
  }

  /**
   * Set a dimension's grade. Optimistically updates the local grade, PUTs it,
   * then reconciles with the server's returned dimension. Reverts on failure.
   */
  async function setGrade(key: string, grade: number): Promise<void> {
    const idx = dimensions.value.findIndex((d) => d.key === key);
    if (idx === -1) {
      return;
    }
    const dim = dimensions.value[idx];
    const clamped = Math.max(0, Math.min(dim.grade_count - 1, Math.round(grade)));
    const previousGrade = dim.grade;

    // Optimistic local update.
    dimensions.value[idx] = { ...dim, grade: clamped };
    error.value = null;

    try {
      const res = await fetch(`/api/difficulty/${encodeURIComponent(key)}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ grade: clamped }),
      });
      if (!res.ok) {
        // Revert.
        dimensions.value[idx] = { ...dim, grade: previousGrade };
        error.value = `Failed to save ${dim.label} (${res.status})`;
        return;
      }
      // Reconcile with the authoritative server value (grade + current_effect).
      const updated = (await res.json()) as DifficultyDimension;
      dimensions.value[idx] = updated;
    } catch (e) {
      dimensions.value[idx] = { ...dim, grade: previousGrade };
      error.value = e instanceof Error ? e.message : "Network error";
    }
  }

  return { dimensions, loading, error, load, setGrade };
});
