import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const target = process.env.HEARTH_SERVER ?? 'http://127.0.0.1:8080';

export default defineConfig({
  plugins: [svelte()],
  server: {
    proxy: {
      '/api': target,
      '/uploads': target,
      '/gateway': { target, ws: true },
    },
  },
});
