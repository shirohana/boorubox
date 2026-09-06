// The import file picker. `pick_library` runs its dialog in Rust because the
// chosen path has to be opened there anyway; import only needs the paths, so it
// uses the dialog plugin directly.
//
// Both entry points need the `dialog:allow-open` permission in
// `src-tauri/capabilities/default.json`; without it the picker rejects.

import { open } from '@tauri-apps/plugin-dialog'

/** Image files to import. Empty when the user cancels. */
export async function pickImportFiles(): Promise<string[]> {
  const picked = await open({
    multiple: true,
    directory: false,
    title: 'Import images',
  })
  return picked ?? []
}

/** A folder to import, recursively. Empty when the user cancels. */
export async function pickImportFolder(): Promise<string[]> {
  const picked = await open({
    multiple: false,
    directory: true,
    title: 'Import a folder',
  })
  return picked == null ? [] : [picked]
}
