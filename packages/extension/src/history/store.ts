// Download History: what the extension keeps about recent captures, and the
// rules over it (spec `download-history`).
//
// One `chrome.storage.local` key per capture, never one array under one key
// (design D2): the worker is killed and restarted at will, and two captures
// seconds apart would otherwise read the same array and write back over each
// other — losing an entry, which is the one thing this extension must not do.

import { deleteBytes, idsWithBytes, putBytes } from './blobs.js'
import { markDelivered, markFailed, sweepPending, type CaptureEntry } from '../delivery/state.js'

export const ENTRY_PREFIX = 'capture:'

/**
 * How many delivered entries are kept, oldest dropped first. Failed entries are
 * never counted here and never dropped: the cap bounds a log, and a failed
 * entry is not a log line, it is an image nobody else holds.
 */
export const DELIVERED_CAP = 200

const BADGE_COLOUR = '#c2410c'

/** An entry plus what only the blob store knows. */
export interface HistoryEntry extends CaptureEntry {
  /** The bytes are still here, so this entry can be retried. */
  hasBytes: boolean
}

function keyFor(id: string) {
  return `${ENTRY_PREFIX}${id}`
}

export async function readEntries(): Promise<CaptureEntry[]> {
  const stored = await chrome.storage.local.get(null)
  return Object.entries(stored)
    .filter(([key]) => key.startsWith(ENTRY_PREFIX))
    .map(([, value]) => value as CaptureEntry)
    .sort((a, b) => b.capturedAt - a.capturedAt)
}

export async function readEntry(id: string): Promise<CaptureEntry | undefined> {
  const stored = await chrome.storage.local.get(keyFor(id))
  return stored[keyFor(id)] as CaptureEntry | undefined
}

/** The history the popup draws: newest first, each row knowing if it can retry. */
export async function readHistory(): Promise<HistoryEntry[]> {
  const [entries, withBytes] = await Promise.all([readEntries(), idsWithBytes()])
  return entries.map((entry) => ({ ...entry, hasBytes: withBytes.has(entry.id) }))
}

/** Write one entry and bring the badge in step. Never touches another entry. */
export async function writeEntry(entry: CaptureEntry): Promise<void> {
  await chrome.storage.local.set({ [keyFor(entry.id)]: entry })
  await refreshBadge()
}

/**
 * The bytes go down before the entry does, and both before the first POST
 * (design D4): the worker can be killed between the capture and the response,
 * and by then the tab may be gone.
 */
export async function beginCapture(entry: CaptureEntry, blob: Blob): Promise<void> {
  await putBytes(entry.id, blob)
  await writeEntry(entry)
}

/** The app took it: the entry is terminal and the bytes are no longer owed. */
export async function recordDelivered(entry: CaptureEntry): Promise<void> {
  await writeEntry(markDelivered(entry))
  await deleteBytes(entry.id)
  await trimDelivered()
}

export async function recordFailed(entry: CaptureEntry, reason: string): Promise<void> {
  await writeEntry(markFailed(entry, reason))
}

/** Fail every entry left in flight by a worker that is gone (design D4). */
export async function sweepInterrupted(): Promise<CaptureEntry[]> {
  const swept = sweepPending(await readEntries())
  for (const entry of swept) {
    await chrome.storage.local.set({ [keyFor(entry.id)]: entry })
  }
  await refreshBadge()
  return swept
}

/** Clear history: what the app confirmed goes, what it did not stays. */
export async function clearDelivered(): Promise<void> {
  const delivered = (await readEntries()).filter((entry) => entry.status === 'delivered')
  await chrome.storage.local.remove(delivered.map((entry) => keyFor(entry.id)))
  await refreshBadge()
}

/** Discard one entry and its bytes, and nothing else. */
export async function discardEntry(id: string): Promise<void> {
  await chrome.storage.local.remove(keyFor(id))
  await deleteBytes(id)
  await refreshBadge()
}

/** The badge counts what is owed, not what was saved (spec `download-history`). */
export async function refreshBadge(): Promise<void> {
  const failed = (await readEntries()).filter((entry) => entry.status === 'failed').length
  await chrome.action.setBadgeText({ text: failed > 0 ? String(failed) : '' })
  if (failed > 0) {
    await chrome.action.setBadgeBackgroundColor({ color: BADGE_COLOUR })
  }
}

async function trimDelivered(): Promise<void> {
  const delivered = (await readEntries()).filter((entry) => entry.status === 'delivered')
  const excess = delivered.slice(DELIVERED_CAP)
  if (excess.length > 0) {
    await chrome.storage.local.remove(excess.map((entry) => keyFor(entry.id)))
  }
}
