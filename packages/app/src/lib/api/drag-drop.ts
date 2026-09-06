// Files dropped on the window.
//
// An HTML5 `drop` event inside a webview hands over `File` objects with no
// filesystem path, and import needs paths. Tauri's own drag-drop event carries
// them, so it is the only usable source here. It is on by default
// (`app.windows[].dragDropEnabled`); turning it off to get HTML5 drag and drop
// would silently kill import by drop.

import type { UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'

export interface FileDropHandlers {
  /** The pointer is over the window carrying files; fires repeatedly. */
  onhover?: () => void
  onleave?: () => void
  ondrop: (paths: string[]) => void
}

export function onFileDrop(handlers: FileDropHandlers): Promise<UnlistenFn> {
  return getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === 'drop') handlers.ondrop(payload.paths)
    else if (payload.type === 'leave') handlers.onleave?.()
    else handlers.onhover?.()
  })
}
