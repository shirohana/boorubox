import type { ImportProgress } from '@boorubox/shared'
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
