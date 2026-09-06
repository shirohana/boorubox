import type { Snippet } from 'svelte'

/**
 * What the current route puts in the top bar (slot map "Toolbar · …"). The bar
 * itself is the frame's — it holds the sidebar toggle and is the drag region
 * (design D13) — so it is never empty, and a route fills the rest of it by
 * handing over a snippet for as long as it is mounted. `$state.raw`: a snippet
 * is a function, and there is nothing inside it to proxy.
 */
class Frame {
  toolbar = $state.raw<Snippet | null>(null)
  /**
   * Slot "Sidebar · filters": the tag list and the rating pills of the current
   * result set. Absent rather than empty (app-shell) — only a screen that has a
   * result set to describe fills it, so /settings shows nav and library alone.
   */
  filters = $state.raw<Snippet | null>(null)
}

export const frame = new Frame()
