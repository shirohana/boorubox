/**
 * Commands reject with a plain string: `AppError` in `src-tauri/src/error.rs`
 * serialises as its `Display` text. The other branches cover the rejections
 * that do not come from a command — a thrown `Error` inside the wrapper, or a
 * value from a plugin — so a caller can always show something.
 */
export function errorText(error: unknown): string {
  if (typeof error === 'string') return error
  if (error instanceof Error) return error.message
  return String(error)
}
