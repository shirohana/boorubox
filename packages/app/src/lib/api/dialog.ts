// The import file picker and the export save picker. `pick_library` runs its
// dialog in Rust because the chosen path has to be opened there anyway; import
// and export only need a path, so both use the dialog plugin directly (design
// D12).
//
// The import pickers need `dialog:allow-open` and the export picker needs
// `dialog:allow-save`, both in `src-tauri/capabilities/default.json`; without
// them the picker rejects.

import { open, save } from '@tauri-apps/plugin-dialog'

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

/**
 * The legacy bundle's `.db` part files (`legacy-bundle-import` design D5): one
 * part is a valid import on its own, so the picker takes more than one. Empty
 * when the user cancels.
 */
export async function pickBundleFiles(): Promise<string[]> {
  const picked = await open({
    multiple: true,
    directory: false,
    title: 'Import a legacy bundle',
    filters: [{ name: 'SQLite database', extensions: ['db', 'sqlite'] }],
  })
  return picked ?? []
}

/**
 * Where a selection's export zip should be written. `null` when the user
 * cancels the dialog, which `export_zip` must never be called with (spec
 * `export-selected`: "Cancelling the dialog SHALL write nothing").
 */
export async function pickExportZipPath(): Promise<string | null> {
  return save({
    title: 'Export selected images',
    defaultPath: 'export.zip',
    filters: [{ name: 'Zip archive', extensions: ['zip'] }],
  })
}

/**
 * Where the library's rules should be written. `null` when the user cancels,
 * which `rules_export` must never be called with.
 */
export async function pickRulesExportPath(): Promise<string | null> {
  return save({
    title: 'Export rules',
    defaultPath: 'rules.json',
    filters: [{ name: 'JSON', extensions: ['json'] }],
  })
}

/**
 * A rules file to import — this app's export or the legacy extension's, which
 * are the same shape (`auto-tag-rules` design D10). `null` when cancelled.
 */
export async function pickRulesImportPath(): Promise<string | null> {
  const picked = await open({
    multiple: false,
    directory: false,
    title: 'Import rules',
    filters: [{ name: 'JSON', extensions: ['json'] }],
  })
  return picked ?? null
}
