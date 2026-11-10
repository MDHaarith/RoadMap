import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  base: './',
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
  server: { port: 5173, strictPort: true },
  build: { target: ['es2021', 'safari15'] },
});
