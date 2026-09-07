import type { CaptureMeta, CaptureWithdrawn, ImageRecord, ImportProgress } from '@boorubox/shared'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/** The event `import_paths` emits while it runs (design D12). */
export const IMPORT_PROGRESS_EVENT = 'import:progress'

/**
 * Subscribes to import progress. Call the returned function when the caller
 * goes away, or the handler outlives the component that made it.
 */
export function onImportProgress(
  handler: (progress: ImportProgress) => void,
): Promise<UnlistenFn> {
  return listen<ImportProgress>(IMPORT_PROGRESS_EVENT, (event) => handler(event.payload))
}

/**
 * The event the capture listener emits for each image it newly stored
 * (`commands.rs`).
 */
export const CAPTURE_STORED_EVENT = 'capture:stored'

/**
 * The event the listener emits when the extension announces a capture it is
 * about to fetch, carrying the `CaptureMeta` it announced (`commands.rs`).
 */
export const CAPTURE_PENDING_EVENT = 'capture:pending'

/**
 * The event that settles an announcement that produced no row — withdrawn by
 * the extension, or refused by the app. A placeholder left up by a missing
 * settle only goes on its timeout.
 */
export const CAPTURE_WITHDRAWN_EVENT = 'capture:withdrawn'

/**
 * Subscribes to captures arriving from the browser extension. Nothing in the
 * webview asks for these — they land while the user is in another window — so a
 * screen showing the library has to be told, or it goes on showing the library
 * as it was when it last searched.
 */
export function onCaptureStored(handler: (image: ImageRecord) => void): Promise<UnlistenFn> {
  return listen<ImageRecord>(CAPTURE_STORED_EVENT, (event) => handler(event.payload))
}

/**
 * Subscribes to the announcement of a capture whose bytes are still being
 * fetched. The placeholder it puts on the library screen is the whole point of
 * announcing: the download is a second of nothing on screen otherwise.
 */
export function onCapturePending(handler: (meta: CaptureMeta) => void): Promise<UnlistenFn> {
  return listen<CaptureMeta>(CAPTURE_PENDING_EVENT, (event) => handler(event.payload))
}

/** Subscribes to announcements that will not become an image. */
export function onCaptureWithdrawn(
  handler: (withdrawn: CaptureWithdrawn) => void,
): Promise<UnlistenFn> {
  return listen<CaptureWithdrawn>(CAPTURE_WITHDRAWN_EVENT, (event) => handler(event.payload))
}
