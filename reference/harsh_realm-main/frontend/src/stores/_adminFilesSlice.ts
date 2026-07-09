import { ref } from "vue";
import type { WorldListItem, YAMLFileEntry } from "../types/api";
import type { AdminSliceCtx } from "./_adminSliceCtx";

export function createAdminFilesSlice(ctx: AdminSliceCtx) {
  const { worldParam, loading, error } = ctx;

  const worlds = ref<WorldListItem[]>([]);
  const yamlFiles = ref<YAMLFileEntry[]>([]);
  const worldMeta = ref<Record<string, string>>({});
  const entitiesByLocation = ref<Record<string, Record<string, unknown>[]>>(
    {},
  );

  async function loadWorldFiles() {
    loading.value = true;
    try {
      const res = await fetch("/api/admin/worlds");
      if (res.ok) worlds.value = await res.json();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function cloneWorld(name: string, newName: string) {
    await fetch(
      `/api/admin/worlds/${encodeURIComponent(name)}/clone?new_name=${encodeURIComponent(newName)}`,
      {
        method: "POST",
      },
    );
  }

  async function exportWorld(name: string) {
    const res = await fetch(
      `/api/admin/worlds/${encodeURIComponent(name)}/export`,
    );
    if (res.ok) {
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${name}_export.zip`;
      a.click();
      URL.revokeObjectURL(url);
    }
  }

  async function deleteWorld(name: string) {
    await fetch(`/api/admin/worlds/${encodeURIComponent(name)}`, {
      method: "DELETE",
    });
  }

  // YAML file content
  async function getYamlContent(path: string): Promise<string> {
    const res = await fetch(`/api/admin/yaml-files/${path}/content`);
    if (!res.ok) return "";
    const data = await res.json();
    return data.content ?? "";
  }

  async function saveYamlContent(path: string, content: string): Promise<void> {
    await fetch(`/api/admin/yaml-files/${path}/content`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content }),
    });
  }

  async function createYamlFile(path: string, content: string): Promise<void> {
    await fetch(`/api/admin/yaml-files/${path}/content`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content }),
    });
  }

  // YAML files
  async function loadYamlFiles() {
    loading.value = true;
    try {
      const res = await fetch("/api/admin/yaml-files");
      if (res.ok) yamlFiles.value = await res.json();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function downloadYaml(path: string) {
    const res = await fetch(`/api/admin/yaml-files/${path}`);
    if (res.ok) {
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = path.split("/").pop() || "file.yaml";
      a.click();
      URL.revokeObjectURL(url);
    }
  }

  async function uploadYaml(path: string, file: File) {
    const formData = new FormData();
    formData.append("file", file);
    await fetch(`/api/admin/yaml-files/${path}`, {
      method: "POST",
      body: formData,
    });
  }

  async function deleteYamlFile(path: string): Promise<boolean> {
    const res = await fetch(`/api/admin/yaml-files/${path}`, {
      method: "DELETE",
    });
    return res.ok;
  }

  async function loadTableStatus(): Promise<
    { path: string; size: number; entry_count: number; has_todo: boolean }[]
  > {
    const res = await fetch("/api/admin/yaml-table-status");
    if (!res.ok) return [];
    return res.json();
  }

  async function downloadTablesZip() {
    const res = await fetch("/api/admin/yaml-tables-zip");
    if (res.ok) {
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "tables.zip";
      a.click();
      URL.revokeObjectURL(url);
    }
  }

  async function uploadTablesZip(
    file: File,
  ): Promise<{ written: string[]; errors: string[] }> {
    const formData = new FormData();
    formData.append("file", file);
    const res = await fetch("/api/admin/yaml-tables-zip", {
      method: "POST",
      body: formData,
    });
    if (!res.ok) return { written: [], errors: ["Upload failed"] };
    return res.json();
  }
  // Entity map overlay
  async function loadEntitiesByLocation() {
    const wp = worldParam();
    const res = await fetch(`/api/admin/entities/by-location${wp}`);
    if (res.ok) entitiesByLocation.value = await res.json();
  }
  // World meta
  async function loadWorldMeta() {
    loading.value = true;
    try {
      const wp = worldParam();
      const res = await fetch(`/api/admin/world-meta${wp}`);
      if (res.ok) worldMeta.value = await res.json();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function setWorldMeta(key: string, value: string) {
    const wp = worldParam();
    await fetch(`/api/admin/world-meta/${encodeURIComponent(key)}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ value }),
    });
  }

  return {
    worlds,
    yamlFiles,
    worldMeta,
    entitiesByLocation,
    loadWorldFiles,
    cloneWorld,
    exportWorld,
    deleteWorld,
    getYamlContent,
    saveYamlContent,
    createYamlFile,
    loadYamlFiles,
    downloadYaml,
    uploadYaml,
    deleteYamlFile,
    loadTableStatus,
    downloadTablesZip,
    uploadTablesZip,
    loadEntitiesByLocation,
    loadWorldMeta,
    setWorldMeta,
  };
}
