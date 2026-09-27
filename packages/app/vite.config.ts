import { defineConfig } from 'vite'
import { sveltekit } from '@sveltejs/kit/vite'
import tailwindcss from '@tailwindcss/vite'
import process from 'node:process'

const host = process.env.TAURI_DEV_HOST

// Vite options tailored for Tauri development, applied in `tauri dev` and `tauri build`.
// Run vitest from this directory only: the SvelteKit plugin sets vite `root` to
// process.cwd(), so a runner started at the workspace root would scan every package.
export default defineConfig(() => ({
  plugins: [tailwindcss(), sveltekit()],
  // Svelte's own recipe for mounting a component under Vitest (design D7 of
  // `tag-row-and-inspector-fixes`): unguarded, `svelte` resolves its server
  // build here, where `mount` throws rather than rendering.
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  // prevent Vite from obscuring rust errors
  clearScreen: false,
  // tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || '127.0.0.1',
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
}))
