import type { FileSchema, FieldDef, DataAccessor } from "./types";

/** Cached schemas loaded from the backend. */
let _schemas: FileSchema[] | null = null;
let _loading: Promise<FileSchema[]> | null = null;

interface RawSchema {
  pattern: string;
  label: string;
  meta_fields?: RawField[];
  widget: string;
  accessor: RawAccessor;
  fields: RawField[];
  entry_key?: string;
  sortable?: boolean;
  allow_add?: boolean;
  allow_delete?: boolean;
}

interface RawField {
  key: string;
  type: string;
  label?: string;
  width?: string;
  options?: string[];
}

interface RawAccessor {
  kind: string;
  key?: string;
  path?: string[];
  tiers?: string[];
}

function parseAccessor(raw: RawAccessor): DataAccessor {
  switch (raw.kind) {
    case "root":
      return { kind: "root" };
    case "key":
      return { kind: "key", key: raw.key! };
    case "nested":
      return { kind: "nested", path: raw.path! };
    case "tiered":
      return { kind: "tiered", tiers: raw.tiers! };
    default:
      return { kind: "root" };
  }
}

function parseField(raw: RawField): FieldDef {
  return {
    key: raw.key,
    type: raw.type as FieldDef["type"],
    label: raw.label ?? raw.key,
    width: raw.width as FieldDef["width"],
    options: raw.options,
  };
}

function parseSchema(raw: RawSchema): FileSchema {
  return {
    pattern: raw.pattern,
    label: raw.label,
    metaFields: raw.meta_fields?.map(parseField),
    widget: {
      widget: raw.widget,
      accessor: parseAccessor(raw.accessor),
      fields: (raw.fields ?? []).map(parseField),
      entryKey: raw.entry_key,
      sortable: raw.sortable,
      allowAdd: raw.allow_add,
      allowDelete: raw.allow_delete,
    },
  };
}

/** Load schemas from backend. Caches after first successful fetch. */
export async function loadSchemas(): Promise<FileSchema[]> {
  if (_schemas) return _schemas;
  if (_loading) return _loading;

  _loading = (async () => {
    try {
      const res = await fetch("/api/admin/editor-schemas");
      if (!res.ok) return [];
      const raw: RawSchema[] = await res.json();
      _schemas = raw.map(parseSchema);
      return _schemas;
    } catch {
      return [];
    } finally {
      _loading = null;
    }
  })();

  return _loading;
}

/** Invalidate the cache so schemas are re-fetched on next access. */
export function invalidateSchemaCache(): void {
  _schemas = null;
  _loading = null;
}

/**
 * Find the matching schema for a given file path.
 * Matches are checked in order; first match wins (more specific patterns first).
 * Returns null if schemas haven't been loaded yet — call loadSchemas() first.
 */
export function getSchemaForFile(path: string): FileSchema | null {
  if (!_schemas) return null;
  const normalizedPath = path.replace(/\\/g, "/");
  // Sort by specificity (longer pattern first)
  const sorted = [..._schemas].sort((a, b) => {
    const patternA = typeof a.pattern === "string" ? a.pattern : a.pattern.source;
    const patternB = typeof b.pattern === "string" ? b.pattern : b.pattern.source;
    return patternB.length - patternA.length;
  });

  for (const schema of sorted) {
    if (typeof schema.pattern === "string") {
      if (normalizedPath.includes(schema.pattern)) return schema;
    } else {
      if (schema.pattern.test(normalizedPath)) return schema;
    }
  }
  return null;
}
