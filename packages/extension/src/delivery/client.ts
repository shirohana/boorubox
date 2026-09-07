// Talking to the app: one POST per delivery attempt, one GET for the popup's
// indicator. Nothing here decides what a capture's state becomes — that is
// `state.ts` — so this module can be read as "what the app answered".

import type { CaptureMeta, StatusResponse } from '@boorubox/shared'

/**
 * Design D15 pins these two part names; the app answers 400 to any other, so
 * they are a contract rather than a convention.
 */
const FILE_FIELD = 'file'
const META_FIELD = 'meta'

/**
 * The app is on loopback. A request still open after this is a wedged app, and
 * the capture is better off failed and retryable than pending for ever.
 */
export const REQUEST_TIMEOUT_MS = 30_000

export type DeliveryOutcome
  = | { delivered: true }
    | { delivered: false, reason: string }

/**
 * Hand one capture to the app.
 *
 * The body is `FormData` with no hand-set `Content-Type`: the boundary has to
 * come from the form, and letting the browser build the request is also what
 * makes it send `Origin: chrome-extension://…`, the header the app's origin
 * layer requires (§5).
 */
export async function postCapture(
  endpoint: string,
  meta: CaptureMeta,
  blob: Blob,
): Promise<DeliveryOutcome> {
  const body = new FormData()
  body.append(FILE_FIELD, blob, meta.id)
  body.append(META_FIELD, JSON.stringify(meta))

  let response: Response
  try {
    response = await fetch(endpoint, {
      method: 'POST',
      body,
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    })
  } catch (error) {
    return { delivered: false, reason: unreachableReason(error) }
  }

  if (response.ok) {
    return { delivered: true }
  }
  return { delivered: false, reason: await refusalReason(response) }
}

export type ConnectionState
  = | { state: 'connected', imageCount: number, libraryPath: string }
    | { state: 'no-library' }
    | { state: 'unreachable' }

/**
 * What the popup's indicator shows. `GET /status` answers 503 when the app runs
 * with no library open, which is a different thing from the app not running:
 * captures fail in both cases, but only one of them is fixed by opening a
 * library.
 */
export async function probeStatus(endpoint: string): Promise<ConnectionState> {
  let response: Response
  try {
    response = await fetch(endpoint, { signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS) })
  } catch {
    return { state: 'unreachable' }
  }

  if (response.status === 503) {
    return { state: 'no-library' }
  }
  if (!response.ok) {
    return { state: 'unreachable' }
  }
  try {
    const status = (await response.json()) as StatusResponse
    return {
      state: 'connected',
      imageCount: status.imageCount,
      libraryPath: status.libraryPath,
    }
  } catch {
    // Answering 200 with something that is not a status is not this app.
    return { state: 'unreachable' }
  }
}

/** The app names why it refused; falling back to the status keeps a reason always. */
async function refusalReason(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { error?: unknown }
    if (typeof body.error === 'string' && body.error !== '') {
      return body.error
    }
  } catch {
    // Not the app's JSON refusal; the status line is what is left.
  }
  return `HTTP ${response.status} ${response.statusText}`.trim()
}

function unreachableReason(error: unknown): string {
  if (error instanceof Error && (error.name === 'TimeoutError' || error.name === 'AbortError')) {
    return `The app did not answer within ${REQUEST_TIMEOUT_MS / 1000} seconds`
  }
  return 'Could not reach the app'
}
