// Pure builders that turn widget editor state back into raw YAML document shapes.
// Extracted from useSchemaFormEditor to keep files under 400 lines.
import type { EntryRecord, FileSchema } from "./types";

export function rebuildWeightedEntries(
  schema: FileSchema | null,
  entries: EntryRecord[],
): unknown[] {
  if (!schema) return entries;

  const fields = schema.widget.fields;
  const isSimpleResult = fields.length === 2 && fields.some((f) => f.key === "result");
  const isStringList = fields.length === 1 && fields[0].key === "value";
  const hasResultField = fields.some((f) => f.key !== "weight" && f.key !== "result");

  if (isStringList) {
    return entries.map((e) => e.value);
  }

  if (isSimpleResult) {
    return entries.map((e) => ({ weight: e.weight, result: e.result }));
  }

  if (hasResultField) {
    // Rebuild nested result objects
    return entries.map((e) => {
      const weight = e.weight;
      const result: Record<string, unknown> = {};
      for (const f of fields) {
        if (f.key === "weight") continue;
        result[f.key] = e[f.key];
      }
      return { weight, result };
    });
  }

  return entries;
}

export function rebuildRangeEntries(entries: EntryRecord[]): unknown[] {
  return entries.map((e) => {
    const { low, high, ...rest } = e;
    return { min: low, max: high, ...rest };
  });
}

export function rebuildMatrix(
  matrixData: Record<string, Record<string, unknown>>,
  matrixColKeys: string[],
): Record<string, unknown> {
  // Check if columns are flattened (contain dots)
  const hasDots = matrixColKeys.some((c) => c.includes("."));
  if (!hasDots) return matrixData;

  // Unflatten: "chaos_1.yes_threshold" -> { chaos_1: { yes_threshold: val } }
  const result: Record<string, Record<string, Record<string, unknown>>> = {};
  for (const [rowKey, rowData] of Object.entries(matrixData)) {
    result[rowKey] = {};
    for (const [flatCol, val] of Object.entries(rowData)) {
      const [col, subKey] = flatCol.split(".");
      if (!result[rowKey][col]) result[rowKey][col] = {};
      result[rowKey][col][subKey] = val;
    }
  }
  return result;
}

export function rebuildGrouped(
  groupedData: { name: string; entries: EntryRecord[] }[],
): unknown[] {
  return groupedData.map((g) => {
    const elements = g.entries.map((e) => {
      if (Object.keys(e).length === 1 && "element" in e) return e.element;
      return e;
    });
    const obj: Record<string, unknown> = { name: g.name, elements };
    // Preserve extra group fields like weight
    const raw = g as unknown as Record<string, unknown>;
    if (raw.weight !== undefined) obj.weight = raw.weight;
    return obj;
  });
}
