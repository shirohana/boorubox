import { defineConfig } from 'vitest/config'

// One root runner; each package under packages/* supplies its own vite or
// vitest config (the app needs the Svelte plugin, the others do not).
export default defineConfig({
  test: {
    projects: ['packages/*'],
  },
})
