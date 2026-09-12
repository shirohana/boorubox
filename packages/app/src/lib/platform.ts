/**
 * Whether the webview is inside the macOS window, read off the attribute
 * `app.html` stamps before the first paint (app-shell design D13). Read once:
 * the platform does not change under a running page.
 */
export const isMacos
  = typeof document !== 'undefined' && document.documentElement.dataset.platform === 'macos'

/**
 * The value for `data-tauri-drag-region` on the surfaces the window is dragged
 * by; `undefined` renders no attribute at all.
 *
 * macOS only. There the title bar is overlaid (`titleBarStyle: Overlay`, which
 * tauri compiles for macOS alone), so the page has to offer a drag surface.
 * Windows keeps its native title bar and needs none — and there a drag region
 * is only harm: tao starts the drag by simulating a title-bar click, so the
 * webview sees focus lost and regained and never a mouse-up, the text
 * selection flickers, an open popover closes with the focus, and the window
 * crawls until another window is focused (tauri-apps/tauri#10767, open as of
 * 2026-09). Putting the attribute back unconditionally brings that back.
 */
export const windowDragRegion: '' | undefined = isMacos ? '' : undefined
