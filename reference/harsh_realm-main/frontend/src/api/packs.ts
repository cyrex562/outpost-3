import type { PackManifest } from "../types/api";

export async function fetchPacks(): Promise<PackManifest[]> {
  const response = await fetch("/api/packs");
  if (!response.ok) {
    throw new Error(`Failed to load packs: ${response.status}`);
  }
  return (await response.json()) as PackManifest[];
}
