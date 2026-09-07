// What a capture's delivery state is, and the only ways it may change
// (design D4). Pure on purpose: no `chrome`, no storage, no clock beyond what
// is handed in — the guarantee this file encodes is the one the whole extension
// exists for, and it should be readable without a browser.

import type { SiteAdapterRecord } from '@boorubox/shared'

export type DeliveryStatus = 'pending' | 'delivered' | 'failed'

/** One capture, as it is stored under `capture:<id>` (design D2). */
export interface CaptureEntry {
  /** The caller-generated UUID the app is idempotent on. */
  id: string
  status: DeliveryStatus
  imageUrl: string
  pageUrl: string
  pageTitle: string
  /** Epoch milliseconds. */
  capturedAt: number
  /** Bytes of the captured image, for the popup's row. */
  size: number
  /** 128 px JPEG data URL (design D3). */
  thumbnail: string
  /**
   * What the site adapter read off the page, if anything. Kept on the entry
   * because a retry posts the same capture, and the page it came from may be
   * closed by then (design D5).
   */
  adapter?: SiteAdapterRecord
  /** Why the app did not take it; absent unless `failed`. */
  reason?: string
}

export const SWEEP_REASON
  = 'The browser stopped the extension before the app answered'

export function createEntry(fields: Omit<CaptureEntry, 'status' | 'reason'>): CaptureEntry {
  return { ...fields, status: 'pending' }
}

/**
 * A retry puts an entry back in flight. Refused on a delivered entry: the app
 * already holds that image, and posting it again would be asking for the
 * duplicate its idempotency exists to absorb.
 */
export function beginDelivery(entry: CaptureEntry): CaptureEntry {
  if (entry.status === 'delivered') {
    return entry
  }
  const { reason: _dropped, ...rest } = entry
  return { ...rest, status: 'pending' }
}

/** The app answered 2xx. Terminal: nothing moves an entry out of here. */
export function markDelivered(entry: CaptureEntry): CaptureEntry {
  const { reason: _dropped, ...rest } = entry
  return { ...rest, status: 'delivered' }
}

/** Anything else. The bytes stay; the entry stays retryable. */
export function markFailed(entry: CaptureEntry, reason: string): CaptureEntry {
  if (entry.status === 'delivered') {
    return entry
  }
  return { ...entry, status: 'failed', reason }
}

/**
 * Run at worker start. An entry still `pending` belonged to a worker that is
 * gone, and there is no way to learn what happened to its request: marking it
 * failed risks one duplicate delivery, which the app absorbs on the UUID, while
 * leaving it pending risks a capture nobody ever retries (design D4).
 */
export function sweepPending(entries: CaptureEntry[]): CaptureEntry[] {
  return entries
    .filter((entry) => entry.status === 'pending')
    .map((entry) => markFailed(entry, SWEEP_REASON))
}
