import { defineConfig } from 'vitest/config'

// Kept separate from vite.config.ts so tests never load the web-extension
// build plugin.
export default defineConfig({
  test: { environment: 'node' },
})
