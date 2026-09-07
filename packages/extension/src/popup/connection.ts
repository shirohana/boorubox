// The connection line. Three states, because they need three different things
// from the user: nothing, open a library, start the app (spec
// `capture-delivery`, "Connection indicator").

import type { ConnectionState } from '../delivery/client.js'

/** Polled while the popup is open and never in the background (design D7). */
export const POLL_INTERVAL_MS = 3000

export function connectionText(state: ConnectionState): string {
  switch (state.state) {
    case 'connected':
      return `Connected · ${state.imageCount.toLocaleString()} images`
    case 'no-library':
      return 'BooruBox is running, but no library is open'
    case 'unreachable':
      return 'BooruBox is not running · captures will be kept and can be retried'
  }
}

export function renderConnection(state: ConnectionState): HTMLElement {
  const line = document.createElement('p')
  line.className = `connection connection--${state.state}`
  line.dataset.state = state.state

  const dot = document.createElement('span')
  dot.className = 'dot'
  line.append(dot, document.createTextNode(connectionText(state)))
  return line
}
