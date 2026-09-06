// Whole-app scale, the way a browser zooms a page: the webview's zoom factor,
// stepped by Cmd/Ctrl with `=`, `-` and `0`. Session state, not a setting —
// the factor lives in the webview and resets with the window.

import { getCurrentWebview } from '@tauri-apps/api/webview'

export const ZOOM_MIN = 0.5
export const ZOOM_MAX = 2
export const ZOOM_STEP = 0.1

/** The factor after one step in `direction`, clamped; `0` resets. Pure, for the test. */
export function zoomStep(current: number, direction: -1 | 0 | 1): number {
  if (direction === 0) return 1
  const next = Math.round((current + direction * ZOOM_STEP) * 100) / 100
  return Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, next))
}

let factor = 1

/** Needs `core:webview:allow-set-webview-zoom` in the capability, or it rejects. */
export async function zoomBy(direction: -1 | 0 | 1): Promise<void> {
  factor = zoomStep(factor, direction)
  await getCurrentWebview().setZoom(factor)
}
