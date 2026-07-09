<script setup lang="ts">
import { ref } from "vue";
import { useAdminStore } from "../../stores/admin";

const props = defineProps<{
  table: string;
}>();

const emit = defineEmits<{
  imported: [];
}>();

const store = useAdminStore();
const statusMsg = ref("");
const importing = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);
const importFormat = ref<"json" | "csv">("json");
const previewData = ref<Record<string, unknown>[] | null>(null);
const pendingFile = ref<File | null>(null);

function wp(): string {
  return store.worldParam();
}

async function handleExportJson() {
  try {
    const res = await fetch(`/api/admin/export/${props.table}?format=json${wp() ? "&" + wp().slice(1) : ""}`);
    if (!res.ok) {
      statusMsg.value = "Export failed.";
      return;
    }
    const data = await res.json();
    const rows = data.rows || [];
    const blob = new Blob([JSON.stringify(rows, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${props.table}.json`;
    a.click();
    URL.revokeObjectURL(url);
    statusMsg.value = `Exported ${rows.length} rows.`;
  } catch {
    statusMsg.value = "Export failed.";
  }
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleExportCsv() {
  try {
    const res = await fetch(`/api/admin/export/${props.table}?format=csv${wp() ? "&" + wp().slice(1) : ""}`);
    if (!res.ok) {
      statusMsg.value = "Export failed.";
      return;
    }
    const text = await res.text();
    const blob = new Blob([text], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${props.table}.csv`;
    a.click();
    URL.revokeObjectURL(url);
    statusMsg.value = "CSV exported.";
  } catch {
    statusMsg.value = "Export failed.";
  }
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

function startImport(format: "json" | "csv") {
  importFormat.value = format;
  previewData.value = null;
  pendingFile.value = null;
  fileInput.value?.click();
}

async function handleFileSelected(event: Event) {
  const input = event.target as HTMLInputElement;
  if (!input.files?.length) return;
  const file = input.files[0];
  pendingFile.value = file;

  try {
    const text = await file.text();
    if (importFormat.value === "json") {
      const data = JSON.parse(text);
      const rows = Array.isArray(data) ? data : [data];
      previewData.value = rows.slice(0, 3);
      statusMsg.value = `Preview: ${rows.length} row(s). Click Confirm to import.`;
    } else {
      const lines = text.trim().split("\n");
      statusMsg.value = `Preview: ${Math.max(0, lines.length - 1)} row(s). Click Confirm to import.`;
      previewData.value = [{ _info: `${lines.length - 1} CSV rows` }];
    }
  } catch {
    statusMsg.value = "Failed to parse file.";
    previewData.value = null;
  }
  input.value = "";
}

async function confirmImport() {
  if (!pendingFile.value) return;
  importing.value = true;
  try {
    if (importFormat.value === "json") {
      const text = await pendingFile.value.text();
      const data = JSON.parse(text);
      const rows = Array.isArray(data) ? data : [data];
      const res = await fetch(
        `/api/admin/import/${props.table}${wp() ? wp() : ""}`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ rows }),
        }
      );
      const result = await res.json();
      statusMsg.value = `Imported ${result.imported} rows.`;
      if (result.errors?.length) {
        statusMsg.value += ` (${result.errors.length} errors)`;
      }
    } else {
      const formData = new FormData();
      formData.append("file", pendingFile.value);
      const res = await fetch(
        `/api/admin/import/${props.table}/csv${wp() ? wp() : ""}`,
        { method: "POST", body: formData }
      );
      const result = await res.json();
      statusMsg.value = `Imported ${result.imported} rows.`;
    }
    previewData.value = null;
    pendingFile.value = null;
    emit("imported");
  } catch (e) {
    statusMsg.value = `Import failed: ${e}`;
  } finally {
    importing.value = false;
  }
  setTimeout(() => { statusMsg.value = ""; }, 5000);
}

function cancelImport() {
  previewData.value = null;
  pendingFile.value = null;
  statusMsg.value = "";
}
</script>

<template>
  <div class="flex items-center gap-2 py-1">
    <input ref="fileInput" type="file" accept=".json,.csv" class="hidden" @change="handleFileSelected" />

    <button
      class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200 hover:bg-gray-800"
      @click="handleExportJson"
    >Export JSON</button>
    <button
      class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200 hover:bg-gray-800"
      @click="handleExportCsv"
    >Export CSV</button>
    <button
      class="px-2 py-0.5 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
      @click="startImport('json')"
    >Import JSON</button>
    <button
      class="px-2 py-0.5 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
      @click="startImport('csv')"
    >Import CSV</button>

    <template v-if="previewData">
      <button
        class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        :disabled="importing"
        @click="confirmImport"
      >{{ importing ? "Importing..." : "Confirm" }}</button>
      <button
        class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
        @click="cancelImport"
      >Cancel</button>
    </template>

    <span v-if="statusMsg" class="text-xs text-green-400 ml-1">{{ statusMsg }}</span>
  </div>
</template>
