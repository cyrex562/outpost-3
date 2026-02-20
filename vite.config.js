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
    // Output built CSS/JS into the static directory that Actix serves
    outDir: 'static',
    emptyOutDir: false, // Don't wipe source CSS files in static/css/
    sourcemap: true,
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, 'crates/outpost-server/static/js/main.js'),
      },
      output: {
        entryFileNames: 'js/[name].js',
        chunkFileNames: 'js/[name].js',
        assetFileNames: 'css/[name].[ext]',
      },
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
      '/colony': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
});
