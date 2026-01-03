import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
  root: 'static',
  base: '/static/',
  build: {
    outDir: '../dist/static',
    emptyOutDir: true,
    sourcemap: true,
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, 'static/js/main.js'),
      },
      output: {
        entryFileNames: 'js/[name].js',
        chunkFileNames: 'js/[name].js',
        assetFileNames: '[ext]/[name].[ext]',
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
