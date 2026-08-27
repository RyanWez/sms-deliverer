import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 1420,
    strictPort: true,
  },
  build: { target: 'es2022' },
  resolve: {
    alias: {
      '$lib': resolve(__dirname, 'src/lib'),
    },
    extensions: ['.mjs', '.js', '.ts', '.svelte.ts', '.svelte', '.json'],
  }
});
