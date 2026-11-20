import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const target = process.env.HEARTH_SERVER ?? 'http://127.0.0.1:8080';
const build = process.env.GITHUB_SHA ? `${process.env.GITHUB_SHA.slice(0, 12)}-${Date.now()}` : String(Date.now());

export default defineConfig({
  plugins: [
    svelte(),
    {
      name: 'hearth-version',
      generateBundle() {
        this.emitFile({ type: 'asset', fileName: 'version.json', source: JSON.stringify({ build }) });
      },
    },
  ],
  define: { __BUILD_ID__: JSON.stringify(build) },
  server: {
    proxy: {
      '/api': target,
      '/uploads': target,
      '/gateway': { target, ws: true },
    },
  },
});
