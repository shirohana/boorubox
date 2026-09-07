// One capture, start to finish: bytes off the page, context off the page,
// thumbnail, entry, POST, and what the entry becomes. The chrome event
// listeners that call this live in `index.ts`; everything decidable lives here.

import type { CaptureMeta } from '@boorubox/shared'

import { captureImageBytes } from './capture.js'
import { capturesEndpoint, getPort } from '../settings.js'
import { postCapture } from '../delivery/client.js'
import { beginDelivery, createEntry, type CaptureEntry } from '../delivery/state.js'
import { makeThumbnail } from '../history/thumbnail.js'
import { getBytes } from '../history/blobs.js'
import {
  beginCapture,
  readEntries,
  readEntry,
  recordDelivered,
  recordFailed,
  writeEntry,
} from '../history/store.js'
import { EXTRACT_CONTEXT, type ExtractContextResult, type PageContext } from '../messages.js'
import { notify } from '../notify.js'

/** §5's wording: the one thing the user can do about a failed delivery. */
const FAILED_MESSAGE = 'Open BooruBox to receive this image'

const MISSING_BYTES = 'The captured bytes are no longer in the browser'

export interface ClickedImage {
  tabId: number
  imageUrl: string
  pageUrl: string
  pageTitle: string
}

/**
 * Capture the clicked image and hand it to the app.
 *
 * Both routes failing is the one case that leaves no entry: there are no bytes
 * to keep, so there is nothing to retry and nothing to show (spec
 * `capture-delivery`, "Both routes fail").
 */
export async function captureAndDeliver(clicked: ClickedImage): Promise<void> {
  let blob: Blob
  try {
    blob = await captureImageBytes(clicked.tabId, clicked.imageUrl, clicked.pageUrl)
  } catch (error) {
    notify(`Could not capture that image: ${reasonOf(error)}`)
    return
  }

  const page = await askThePage(clicked)
  const entry = createEntry({
    id: crypto.randomUUID(),
    imageUrl: clicked.imageUrl,
    pageUrl: clicked.pageUrl,
    // The tab's title is the fallback, not the source: it mirrors
    // `document.title`, and a site that never rewrites that on a route still
    // names the screen the user came from.
    pageTitle: page?.pageTitle || clicked.pageTitle,
    capturedAt: Date.now(),
    size: blob.size,
    thumbnail: await makeThumbnail(blob),
    ...(page?.record ? { adapter: page.record } : {}),
  })

  await beginCapture(entry, blob)
  await deliver(entry, blob)
}

/**
 * Post the bytes already kept, under the original id, and never ask the image
 * host again (design D5): the page may be closed, the URL may be single-use,
 * and the bytes on disk are what the user saw.
 */
export async function retryCapture(id: string): Promise<void> {
  const entry = await readEntry(id)
  if (!entry || entry.status === 'delivered') {
    return
  }

  const blob = await getBytes(id)
  if (!blob) {
    await recordFailed(entry, MISSING_BYTES)
    return
  }

  await writeEntry(beginDelivery(entry))
  await deliver(entry, blob)
}

export async function retryAllFailed(): Promise<void> {
  const failed = (await readEntries()).filter((entry) => entry.status === 'failed')
  for (const entry of failed) {
    await retryCapture(entry.id)
  }
}

async function deliver(entry: CaptureEntry, blob: Blob): Promise<void> {
  const outcome = await postCapture(capturesEndpoint(await getPort()), metaOf(entry), blob)
  if (outcome.delivered) {
    await recordDelivered(entry)
    return
  }
  await recordFailed(entry, outcome.reason)
  // Exactly one per failed delivery (design D15). The badge is the reliable
  // signal; this is the one that arrives while the user is looking elsewhere.
  notify(FAILED_MESSAGE, entry.id)
}

function metaOf(entry: CaptureEntry): CaptureMeta {
  return {
    id: entry.id,
    imageUrl: entry.imageUrl,
    pageUrl: entry.pageUrl,
    pageTitle: entry.pageTitle,
    capturedAt: entry.capturedAt,
    ...(entry.adapter ? { adapter: entry.adapter } : {}),
  }
}

/**
 * Asked of the tab whichever way the bytes were obtained (design D9). A page
 * with no content script at all answers nothing, and the capture goes with the
 * tab's own title and no record rather than not going.
 */
async function askThePage(clicked: ClickedImage): Promise<PageContext | null> {
  try {
    const page: ExtractContextResult = await chrome.tabs.sendMessage(clicked.tabId, {
      type: EXTRACT_CONTEXT,
      imageUrl: clicked.imageUrl,
    })
    return page ?? null
  } catch {
    return null
  }
}

function reasonOf(error: unknown): string {
  return error instanceof Error ? error.message : 'Unknown error'
}
