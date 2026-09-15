// Whether the window is full screen. Read back from the OS after every
// change rather than assumed, because `setFullscreen` can be refused; and
// re-readable on demand (`refresh`) because a resize can leave full screen
// without the app's own toggle running — see `+layout.svelte`'s `onresize`.

import { getCurrentWindow } from '@tauri-apps/api/window'

class Fullscreen {
  active = $state(false)

  /** Needs `core:window:allow-set-fullscreen` and `core:window:allow-is-fullscreen`. */
  async toggle(): Promise<void> {
    await getCurrentWindow().setFullscreen(!this.active)
    await this.refresh()
  }

  async refresh(): Promise<void> {
    this.active = await getCurrentWindow().isFullscreen()
  }
}

export const fullscreen = new Fullscreen()
