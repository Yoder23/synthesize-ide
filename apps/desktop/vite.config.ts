import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  // Tauri serves the bundled frontend from its asset protocol rather than a
  // web origin. Relative URLs are required for the production window to load
  // its JS/CSS instead of rendering a blank webview.
  base: './',
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ['VITE_', 'TAURI_']
});
