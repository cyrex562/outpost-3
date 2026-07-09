// State and YAML-document codec for SchemaFormEditor (extracted to keep files under 400 lines).
import { ref, computed, watch } from "vue";
import yaml from "js-yaml";
import { getSchemaForFile, loadSchemas } from "./schemas";
import {
  rebuildGrouped,
  rebuildMatrix,
  rebuildRangeEntries,
  rebuildWeightedEntries,
} from "./_schemaDocBuilders";
import type { FileSchema, EntryRecord } from "./types";

interface SchemaFormEmit {
  (event: "saved"): void;
  (event: "status", msg: string): void;
}

export function useSchemaFormEditor(filePath: () => string, emit: SchemaFormEmit) {
  const schema = ref<FileSchema | null>(null);
  const rawDoc = ref<Record<string, unknown>>({});
  const entries = ref<EntryRecord[]>([]);
  const tieredData = ref<Record<string, { name?: string; items: EntryRecord[] }>>({});
  const matrixData = ref<Record<string, Record<string, unknown>>>({});
  const matrixRowKeys = ref<string[]>([]);
  const matrixColKeys = ref<string[]>([]);
  const groupedData = ref<{ name: string; entries: EntryRecord[] }[]>([]);
  const loading = ref(false);
  const editingRaw = ref(false);
  const rawContent = ref("");
  const showPasteImporter = ref(false);

  const widgetName = computed(() => schema.value?.widget.widget ?? "");

  const tierKeys = computed<string[]>(() => {
    const accessor = schema.value?.widget.accessor;
    if (accessor?.kind === "tiered") return accessor.tiers;
    return [];
  });

  watch(() => filePath(), () => { loadFile(); }, { immediate: true });

  async function loadFile() {
    await loadSchemas();
    schema.value = getSchemaForFile(filePath());
    if (!schema.value) {
      emit("status", "No schema found for this file type.");
      return;
    }

    loading.value = true;
    try {
      const res = await fetch(`/api/admin/yaml-files/${filePath()}/content`);
      if (!res.ok) { emit("status", "Failed to load file."); return; }
      const data = await res.json();
      rawContent.value = data.content;
      const doc = yaml.load(data.content) as Record<string, unknown> | unknown[];
      parseDoc(doc);
      emit("status", "");
    } catch {
      emit("status", "Failed to load file.");
    } finally {
      loading.value = false;
    }
  }

  function parseDoc(doc: Record<string, unknown> | unknown[]) {
    const s = schema.value;
    if (!s) return;

    const accessor = s.widget.accessor;

    if (accessor.kind === "root") {
      // doc is the array directly
      rawDoc.value = {};
      entries.value = (Array.isArray(doc) ? doc : []) as EntryRecord[];
      return;
    }

    const docObj = (Array.isArray(doc) ? {} : doc) as Record<string, unknown>;
    rawDoc.value = docObj;

    if (accessor.kind === "key") {
      const raw = docObj[accessor.key];
      if (s.widget.widget === "MatrixEditor") {
        parseMatrix(raw as Record<string, Record<string, unknown>>);
      } else if (s.widget.widget === "GroupedListEditor") {
        parseGrouped(raw);
      } else if (s.widget.widget === "WeightedListEditor" || s.widget.widget === "RangeTableEditor") {
        parseEntries(raw);
      } else {
        entries.value = (Array.isArray(raw) ? raw : []) as EntryRecord[];
      }
      return;
    }

    if (accessor.kind === "nested") {
      let current: unknown = docObj;
      for (const p of accessor.path) {
        current = (current as Record<string, unknown>)?.[p];
      }
      entries.value = (Array.isArray(current) ? current : []) as EntryRecord[];
      return;
    }

    if (accessor.kind === "tiered") {
      const result: Record<string, { name?: string; items: EntryRecord[] }> = {};
      for (const tier of accessor.tiers) {
        const tierObj = docObj[tier] as Record<string, unknown> | undefined;
        if (tierObj) {
          result[tier] = {
            name: (tierObj.name as string) ?? "",
            items: (Array.isArray(tierObj.items) ? tierObj.items : []) as EntryRecord[],
          };
        } else {
          result[tier] = { name: "", items: [] };
        }
      }
      tieredData.value = result;
      return;
    }
  }

  function parseEntries(raw: unknown) {
    if (!Array.isArray(raw)) { entries.value = []; return; }
    const s = schema.value;
    if (!s) return;

    // Flatten nested result objects for WeightedListEditor
    const hasResultField = s.widget.fields.some((f) => f.key !== "weight" && f.key !== "result");
    const isSimpleResult = s.widget.fields.length === 2 && s.widget.fields.some((f) => f.key === "result");
    const isStringList = raw.length > 0 && typeof raw[0] === "string";

    if (isStringList) {
      entries.value = raw.map((v) => ({ value: String(v) }));
      return;
    }

    entries.value = raw.map((item: unknown) => {
      if (typeof item !== "object" || item === null) return { value: String(item) };
      const obj = item as Record<string, unknown>;

      if (isSimpleResult) {
        // Flat weighted: { weight, result: "string" }
        return { weight: obj.weight ?? 1, result: obj.result ?? "" };
      }

      if (hasResultField && obj.result && typeof obj.result === "object") {
        // Nested: { weight, result: { type, name, ... } } -> flatten
        const result = obj.result as Record<string, unknown>;
        return { weight: obj.weight ?? 1, ...result };
      }

      // Range tables: { min, max, ... }
      if ("min" in obj && "max" in obj) {
        return { low: obj.min, high: obj.max, ...Object.fromEntries(
          Object.entries(obj).filter(([k]) => k !== "min" && k !== "max")
        ) };
      }

      return obj as EntryRecord;
    });
  }

  function parseMatrix(raw: Record<string, Record<string, unknown>>) {
    if (!raw || typeof raw !== "object") {
      matrixData.value = {};
      matrixRowKeys.value = [];
      matrixColKeys.value = [];
      return;
    }

    matrixRowKeys.value = Object.keys(raw);
    const colSet = new Set<string>();
    for (const row of Object.values(raw)) {
      if (row && typeof row === "object") {
        for (const col of Object.keys(row)) colSet.add(col);
      }
    }
    matrixColKeys.value = [...colSet].sort();

    // For fate_chart, each row is a likelihood, each column is a chaos level
    // and values are objects like {yes_threshold, exceptional_yes, exceptional_no}
    // We need to represent this differently — show sub-keys as separate columns
    const firstRow = Object.values(raw)[0];
    const firstVal = firstRow ? Object.values(firstRow)[0] : null;

    if (firstVal && typeof firstVal === "object" && !Array.isArray(firstVal)) {
      // Nested objects — flatten: row x (col.subkey) matrix
      const subKeys = Object.keys(firstVal as Record<string, unknown>);
      const flatCols: string[] = [];
      const flatMatrix: Record<string, Record<string, unknown>> = {};

      for (const colKey of matrixColKeys.value) {
        for (const subKey of subKeys) {
          flatCols.push(`${colKey}.${subKey}`);
        }
      }

      for (const [rowKey, rowData] of Object.entries(raw)) {
        flatMatrix[rowKey] = {};
        if (rowData && typeof rowData === "object") {
          for (const [colKey, colVal] of Object.entries(rowData)) {
            if (colVal && typeof colVal === "object" && !Array.isArray(colVal)) {
              for (const [subKey, subVal] of Object.entries(colVal as Record<string, unknown>)) {
                flatMatrix[rowKey][`${colKey}.${subKey}`] = subVal;
              }
            }
          }
        }
      }

      matrixData.value = flatMatrix;
      matrixColKeys.value = flatCols;
    } else {
      matrixData.value = raw;
    }
  }

  function parseGrouped(raw: unknown) {
    if (!Array.isArray(raw)) { groupedData.value = []; return; }

    groupedData.value = raw.map((item: unknown) => {
      if (typeof item !== "object" || item === null) return { name: String(item), entries: [] };
      const obj = item as Record<string, unknown>;
      const name = String(obj.name ?? "");
      // Elements can be an array of strings or objects
      const elements = Array.isArray(obj.elements) ? obj.elements : [];
      const entryRecords = elements.map((e: unknown) =>
        typeof e === "string" ? { element: e } : (e as EntryRecord)
      );
      return { name, weight: obj.weight, entries: entryRecords } as { name: string; entries: EntryRecord[] };
    });
  }

  // --- Serialization back to YAML ---

  function buildDoc(): unknown {
    const s = schema.value;
    if (!s) return {};

    const accessor = s.widget.accessor;

    if (accessor.kind === "root") {
      return entries.value;
    }

    const doc = { ...rawDoc.value };

    if (accessor.kind === "key") {
      if (widgetName.value === "MatrixEditor") {
        doc[accessor.key] = rebuildMatrix(matrixData.value, matrixColKeys.value);
      } else if (widgetName.value === "GroupedListEditor") {
        doc[accessor.key] = rebuildGrouped(groupedData.value);
      } else if (widgetName.value === "WeightedListEditor") {
        doc[accessor.key] = rebuildWeightedEntries(schema.value, entries.value);
      } else if (widgetName.value === "RangeTableEditor") {
        doc[accessor.key] = rebuildRangeEntries(entries.value);
      } else {
        doc[accessor.key] = entries.value;
      }
      return doc;
    }

    if (accessor.kind === "nested") {
      let current = doc;
      for (let i = 0; i < accessor.path.length - 1; i++) {
        const k = accessor.path[i];
        if (!current[k] || typeof current[k] !== "object") current[k] = {};
        current = current[k] as Record<string, unknown>;
      }
      current[accessor.path[accessor.path.length - 1]] = entries.value;
      return doc;
    }

    if (accessor.kind === "tiered") {
      for (const tier of accessor.tiers) {
        const td = tieredData.value[tier];
        if (td) {
          doc[tier] = { name: td.name, items: td.items };
        }
      }
      return doc;
    }

    return doc;
  }

  // --- Meta field access ---

  function getMetaValue(key: string): unknown {
    return rawDoc.value[key];
  }

  function setMetaValue(key: string, val: unknown) {
    rawDoc.value = { ...rawDoc.value, [key]: val };
  }

  // --- Save ---

  async function save() {
    const doc = buildDoc();
    const content = yaml.dump(doc, { lineWidth: -1, noRefs: true, sortKeys: false });

    try {
      const res = await fetch(`/api/admin/yaml-files/${filePath()}/content`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content }),
      });
      if (res.ok) {
        rawContent.value = content;
        emit("status", "Saved.");
        emit("saved");
      } else {
        const err = await res.json().catch(() => ({ detail: "unknown error" }));
        emit("status", `Save failed: ${err.detail || "unknown error"}`);
      }
    } catch {
      emit("status", "Save failed.");
    }
  }

  async function saveRaw() {
    try {
      const res = await fetch(`/api/admin/yaml-files/${filePath()}/content`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content: rawContent.value }),
      });
      if (res.ok) {
        editingRaw.value = false;
        await loadFile();
        emit("status", "Saved.");
      } else {
        emit("status", "Save failed.");
      }
    } catch {
      emit("status", "Save failed.");
    }
  }

  function onPasteImport(imported: EntryRecord[]) {
    entries.value = [...entries.value, ...imported];
    showPasteImporter.value = false;
  }

  return {
    schema, entries, tieredData, matrixData, matrixRowKeys, matrixColKeys,
    groupedData, loading, editingRaw, rawContent, showPasteImporter,
    widgetName, tierKeys, loadFile, save, saveRaw, getMetaValue,
    setMetaValue, onPasteImport,
  };
}
