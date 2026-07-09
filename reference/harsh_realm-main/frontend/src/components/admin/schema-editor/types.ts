export type FieldType = "text" | "number" | "select" | "tags" | "textarea" | "boolean";

export interface FieldDef {
  key: string;
  label?: string | undefined;
  type: FieldType;
  options?: string[] | undefined;
  default?: unknown;
  required?: boolean | undefined;
  width?: ("sm" | "md" | "lg" | "full") | undefined;
}

export type DataAccessor =
  | { kind: "root" }
  | { kind: "key"; key: string }
  | { kind: "nested"; path: string[] }
  | { kind: "tiered"; tiers: string[] };

export interface WidgetSchema {
  widget: string;
  fields: FieldDef[];
  accessor: DataAccessor;
  entryKey?: string | undefined;
  sortable?: boolean | undefined;
  allowAdd?: boolean | undefined;
  allowDelete?: boolean | undefined;
}

export interface FileSchema {
  pattern: string | RegExp;
  label: string;
  widget: WidgetSchema;
  metaFields?: FieldDef[] | undefined;
}

export interface EntryRecord {
  [key: string]: unknown;
}
