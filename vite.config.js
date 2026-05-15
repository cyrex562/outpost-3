import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

export default defineConfig({
  root: 'crates/outpost-server',
  base: '/static/',
  plugins: [
    tailwindcss(),
  ],
  build: {
    outDir: 'static',
    emptyOutDir: false,
    sourcemap: false,
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, 'crates/outpost-server/frontend/main.js'),
      },
      output: {
        // JS entry goes to static/js/main.js (not needed but harmless)
        entryFileNames: 'js/[name].js',
        chunkFileNames: 'js/[name].js',
        // CSS extracted from the JS entry: rename input.css → main.css
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === 'input.css') return 'css/main.css';
          return 'css/[name].[ext]';
        },
      },
    },
  },
});
