// Design D6: restarting mid-import would abandon a migration with no report,
// so the confirmation refuses while work is in flight and says what is
// running. The webview already knows what is running — the `Imports` queue
// and pending captures — so this takes their counts rather than reading
// either store itself, which is what lets `UpdateDialog.svelte` derive them
// with `$derived` (CLAUDE.md: never an `$effect` off a wholesale-reassigned
// store field) and this file stay a plain, independently testable function.

/**
 * What is running, in words fit for the refusal — or `null` when nothing is.
 * `runCount` is `imports.runs.length` (running and queued both count: only
 * one run is ever literally `running`, but a queued one is the same
 * operation still going). `pendingCaptureCount` is `pendingCaptures.entries.length`.
 */
export function workInFlight(runCount: number, pendingCaptureCount: number): string | null {
  const running: string[] = []
  if (runCount > 0) running.push('an import')
  if (pendingCaptureCount > 0) running.push('a capture')
  if (running.length === 0) return null
  const verb = running.length > 1 ? 'are' : 'is'
  return `${running.join(' and ')} ${verb} still running`
}
