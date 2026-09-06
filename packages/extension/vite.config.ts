import { defineConfig } from 'vite'
import webExtension from 'vite-plugin-web-extension'

export default defineConfig(({ mode }) => ({
  plugins: [
    webExtension({
      manifest: './src/manifest.json',
      disableAutoLaunch: mode === 'development',
    }),
  ],
  build: {
    outDir: 'dist',
    emptyOutDir: mode !== 'development',
    rollupOptions: {
      output: {
        chunkFileNames: 'chunks/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash][extname]',
      },
    },
  },
}))
