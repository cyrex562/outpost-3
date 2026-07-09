<script setup lang="ts">
import { ref, computed } from "vue";
import yaml from "js-yaml";
import type { FieldDef, EntryRecord } from "./types";

const props = defineProps<{
  fields: FieldDef[];
}>();

const emit = defineEmits<{
  import: [entries: EntryRecord[]];
  close: [];
}>();

type Format = "auto" | "yaml" | "tsv" | "csv";
const format = ref<Format>("auto");
const pasteText = ref("");
const error = ref("");

const preview = computed<EntryRecord[]>(() => {
  if (!pasteText.value.trim()) return [];
  try {
    error.value = "";
    return parse(pasteText.value.trim(), format.value);
  } catch (e) {
    error.value = String(e);
    return [];
  }
});

function parse(text: string, fmt: Format): EntryRecord[] {
  if (fmt === "auto") {
    // Try YAML first, then TSV, then CSV
    try { return parseYaml(text); } catch { /* continue */ }
    if (text.includes("\t")) return parseDSV(text, "\t");
    if (text.includes(",")) return parseDSV(text, ",");
    return parseYaml(text);
  }
  if (fmt === "yaml") return parseYaml(text);
  if (fmt === "tsv") return parseDSV(text, "\t");
  return parseDSV(text, ",");
}

function parseYaml(text: string): EntryRecord[] {
  const doc = yaml.load(text);
  if (Array.isArray(doc)) {
    return doc.map((item) => {
      if (typeof item === "string" || typeof item === "number") {
        return { [props.fields[0]?.key ?? "value"]: item };
      }
      return item as EntryRecord;
    });
  }
  if (doc && typeof doc === "object") {
    return [doc as EntryRecord];
  }
  throw new Error("Could not parse as YAML list");
}

function parseDSV(text: string, sep: string): EntryRecord[] {
  const lines = text.split("\n").filter((l) => l.trim());
  if (lines.length === 0) return [];

  // First line might be headers
  const firstCols = lines[0].split(sep).map((c) => c.trim());
  const fieldKeys = props.fields.map((f) => f.key);
  const hasHeader = firstCols.some((c) => fieldKeys.includes(c));

  let headers: string[];
  let dataLines: string[];

  if (hasHeader) {
    headers = firstCols;
    dataLines = lines.slice(1);
  } else {
    headers = props.fields.slice(0, firstCols.length).map((f) => f.key);
    dataLines = lines;
  }

  return dataLines.map((line) => {
    const cols = line.split(sep).map((c) => c.trim());
    const entry: EntryRecord = {};
    for (let i = 0; i < headers.length && i < cols.length; i++) {
      const field = props.fields.find((f) => f.key === headers[i]);
      if (field?.type === "number") {
        entry[headers[i]] = Number(cols[i]) || 0;
      } else {
        entry[headers[i]] = cols[i];
      }
    }
    return entry;
  });
}

function doImport() {
  emit("import", preview.value);
}
</script>

<template>
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" @click.self="emit('close')">
    <div class="bg-gray-900 border border-gray-700 rounded-lg shadow-xl w-[700px] max-h-[80vh] flex flex-col">
      <div class="flex items-center justify-between px-4 py-2 border-b border-gray-800">
        <h3 class="text-sm font-bold text-gray-300">Paste Import</h3>
        <button class="text-gray-500 hover:text-gray-300" @click="emit('close')">&times;</button>
      </div>

      <div class="p-4 flex-1 overflow-y-auto space-y-3">
        <!-- Format selector -->
        <div class="flex items-center gap-2">
          <label class="text-xs text-gray-500">Format:</label>
          <select
            v-model="format"
            class="bg-gray-800 border border-gray-700 text-gray-200 text-xs rounded px-2 py-0.5"
          >
            <option value="auto">Auto-detect</option>
            <option value="yaml">YAML</option>
            <option value="tsv">TSV</option>
            <option value="csv">CSV</option>
          </select>
        </div>

        <!-- Paste area -->
        <textarea
          v-model="pasteText"
          class="w-full h-40 bg-gray-800 border border-gray-700 text-gray-200 rounded px-3 py-2 text-xs font-mono focus:outline-none focus:border-gray-500 resize-y"
          placeholder="Paste YAML, TSV, or CSV data here..."
          spellcheck="false"
        ></textarea>

        <p v-if="error" class="text-xs text-red-400">{{ error }}</p>

        <!-- Preview -->
        <div v-if="preview.length > 0">
          <p class="text-xs text-gray-500 mb-1">Preview ({{ preview.length }} entries):</p>
          <div class="max-h-48 overflow-y-auto">
            <table class="w-full text-xs">
              <thead>
                <tr class="text-left text-gray-500 border-b border-gray-800">
                  <th v-for="f in fields" :key="f.key" class="px-2 py-0.5">{{ f.label ?? f.key }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(entry, idx) in preview.slice(0, 20)"
                  :key="idx"
                  class="border-b border-gray-800/30"
                >
                  <td v-for="f in fields" :key="f.key" class="px-2 py-0.5 text-gray-400">
                    {{ entry[f.key] ?? '' }}
                  </td>
                </tr>
              </tbody>
            </table>
            <p v-if="preview.length > 20" class="text-xs text-gray-600 mt-1">
              ...and {{ preview.length - 20 }} more entries
            </p>
          </div>
        </div>
      </div>

      <div class="flex justify-end gap-2 px-4 py-2 border-t border-gray-800">
        <button
          class="px-3 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:bg-gray-800"
          @click="emit('close')"
        >Cancel</button>
        <button
          :disabled="preview.length === 0"
          class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 disabled:opacity-50 disabled:cursor-not-allowed"
          @click="doImport"
        >Import {{ preview.length }} entries</button>
      </div>
    </div>
  </div>
</template>
