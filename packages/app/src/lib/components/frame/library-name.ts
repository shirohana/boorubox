/**
 * The display name of a library folder is its basename, derived wherever it is
 * shown and never stored (design D4) — `recent_libraries` derives the same name
 * in Rust for the start screen, and this is the webview's side for the one
 * library that is open.
 *
 * Both separators on purpose: the paths come from the OS the app runs on, and
 * the same build has to read a Windows path.
 */
export function libraryName(path: string | null): string {
  if (!path) return ''
  const trimmed = path.replace(/[/\\]+$/, '')
  const cut = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'))
  return cut === -1 ? trimmed : trimmed.slice(cut + 1)
}
