import type { ImageRecord, ImportProgress } from '@boorubox/shared'
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
 * Subscribes to captures arriving from the browser extension. Nothing in the
 * webview asks for these — they land while the user is in another window — so a
 * screen showing the library has to be told, or it goes on showing the library
 * as it was when it last searched.
 */
export function onCaptureStored(handler: (image: ImageRecord) => void): Promise<UnlistenFn> {
  return listen<ImageRecord>(CAPTURE_STORED_EVENT, (event) => handler(event.payload))
}
