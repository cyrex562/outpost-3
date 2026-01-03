// Simple WASM dist smoke test without external dependencies.
// Checks that Trunk produced expected files and that index.html contains the expected canvas.
// Usage: node bin/wasm_smoke.mjs
import { readFile, stat } from 'node:fs/promises';
import { resolve } from 'node:path';

const distDir = resolve('crates/outpost-client/dist');

async function fileExists(path) {
  try { await stat(path); return true; } catch { return false; }
}

async function main() {
  const indexPath = resolve(distDir, 'index.html');
  if (!(await fileExists(indexPath))) {
    console.error('WASM smoke failed: dist/index.html missing');
    process.exit(1);
  }
  const indexHtml = await readFile(indexPath, 'utf8');
  if (!indexHtml.includes('#outpost-canvas')) {
    console.error('WASM smoke failed: index.html missing #outpost-canvas');
    process.exit(1);
  }

  // Find wasm file (Trunk typically names it by package)
  // Rather than glob, check some common names and fallback to any .wasm reference in html
  const wasmRefMatch = indexHtml.match(/src\s*=\s*"([^"]+\.wasm)"/);
  let wasmPath;
  if (wasmRefMatch) {
    wasmPath = resolve(distDir, wasmRefMatch[1]);
  } else {
    wasmPath = resolve(distDir, 'outpost-client.wasm');
  }
  if (!(await fileExists(wasmPath))) {
    console.error('WASM smoke failed: wasm bundle not found:', wasmPath);
    process.exit(1);
  }

  const wasmStats = await stat(wasmPath);
  if (wasmStats.size <= 1024) {
    console.error('WASM smoke failed: wasm bundle size too small:', wasmStats.size);
    process.exit(1);
  }

  console.log('WASM smoke OK:', {
    index: indexPath,
    wasm: wasmPath,
    wasmSize: wasmStats.size,
  });
}

main().catch((e) => {
  console.error('WASM smoke exception:', e);
  process.exit(1);
});
