import 'fake-indexeddb/auto'

import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { captureAndDeliver, retryCapture } from './deliver.js'
import { getBytes } from '../history/blobs.js'
import { readHistory } from '../history/store.js'
import { CAPTURE_IMAGE, EXTRACT_CONTEXT } from '../messages.js'
import { installFakeChrome, type FakeChrome } from '../test-support/chrome.js'

const CLICKED = {
  tabId: 1,
  imageUrl: 'https://cdn.test/a.png',
  pageUrl: 'https://x.com/alice/status/1',
  pageTitle: 'alice on X',
}

const RECORD = {
  site: 'x',
  fields: { handle: 'alice', postUrl: 'https://x.com/alice/status/1' },
}

/** What a tab with the content script in it answers. */
const PAGE = { record: RECORD, pageTitle: 'Alice (@alice) on X: hello' }

// A one-pixel PNG as `canvas.toDataURL()` would hand it over.
const DATA_URL = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0l'
  + 'EQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='

const PENDING = 'http://127.0.0.1:47201/captures/pending'
const CAPTURES = 'http://127.0.0.1:47201/captures'

let chrome: FakeChrome
let fetchMock: ReturnType<typeof vi.fn>
/** Every URL fetched, so a test can say the image host was never asked. */
let requested: string[]
/**
 * Everything the capture did that leaves the worker, in order: each message to
 * the tab and each URL fetched. The announcement is only worth anything if it
 * happens before the bytes are looked for, and only this shows that.
 */
let steps: string[]

/** The tab answers the canvas capture and the context ask independently. */
function tabAnswers(options: { capture?: unknown, context?: unknown }) {
  chrome.tabs.sendMessage.mockImplementation(async (_id: number, message: { type: string }) => {
    steps.push(message.type)
    if (message.type === CAPTURE_IMAGE) return options.capture
    if (message.type === EXTRACT_CONTEXT) return options.context
    return undefined
  })
}

function appAnswers(status: number) {
  fetchMock.mockImplementation(async (url: string) => {
    record(url)
    if (url.startsWith('http://127.0.0.1')) {
      if (status === 0) throw new TypeError('Failed to fetch')
      if (url.startsWith(PENDING)) return new Response(null, { status: 202 })
      return new Response(JSON.stringify({ id: 'stored' }), { status })
    }
    return new Response(new Blob([new Uint8Array([9, 9, 9])], { type: 'image/png' }))
  })
}

function record(url: string) {
  requested.push(url)
  steps.push(url)
}

function fetchedFor(url: string) {
  return fetchMock.mock.calls.filter(([called]) => String(called).startsWith(url))
}

function postedMeta() {
  const [, init] = fetchedFor(CAPTURES).find(([url]) => String(url) === CAPTURES)!
  return JSON.parse((init.body as FormData).get('meta') as string)
}

function announcedMeta() {
  const [, init] = fetchedFor(PENDING)[0]!
  return JSON.parse(init.body as string)
}

beforeEach(() => {
  chrome = installFakeChrome()
  requested = []
  steps = []
  fetchMock = vi.fn()
  vi.stubGlobal('fetch', fetchMock)
  indexedDB.deleteDatabase('boorubox-bridge')
})

afterEach(() => {
  vi.unstubAllGlobals()
})

it('sends the adapter record with a capture taken from the page', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: PAGE })
  appAnswers(201)

  await captureAndDeliver(CLICKED)

  expect(postedMeta().adapter).toEqual(RECORD)
  expect(requested).toEqual([PENDING, CAPTURES])
})

it('tells the app a capture is coming before it looks for the bytes', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: PAGE })
  appAnswers(201)

  await captureAndDeliver(CLICKED)

  // The page is the one thing that comes first: it is a DOM read, and it gives
  // the announcement the title the placeholder shows (design D5).
  expect(steps).toEqual([EXTRACT_CONTEXT, PENDING, CAPTURE_IMAGE, CAPTURES])
})

it('announces before the image host is asked for anything', async () => {
  tabAnswers({ capture: { error: 'Image not found in DOM' }, context: PAGE })
  appAnswers(201)

  await captureAndDeliver(CLICKED)

  expect(steps).toEqual([EXTRACT_CONTEXT, PENDING, CAPTURE_IMAGE, CLICKED.imageUrl, CAPTURES])
})

it('announces the capture under the id and time it is posted with', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: PAGE })
  appAnswers(201)

  await captureAndDeliver(CLICKED)

  // Same id or the app settles nothing; same `capturedAt` or the placeholder
  // and the row it becomes sort to different places.
  expect(announcedMeta()).toEqual(postedMeta())
  expect(announcedMeta().id).toBe((await readHistory())[0]!.id)
})

it('delivers the capture when the announcement never got through', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: PAGE })
  fetchMock.mockImplementation(async (url: string) => {
    record(url)
    if (url.startsWith(PENDING)) throw new TypeError('Failed to fetch')
    return new Response(JSON.stringify({ id: 'stored' }), { status: 201 })
  })

  await captureAndDeliver(CLICKED)

  expect((await readHistory())[0]!.status).toBe('delivered')
  expect(requested).toEqual([PENDING, CAPTURES])
})

it('sends the adapter record with a capture that had to be fetched', async () => {
  tabAnswers({ capture: { error: 'Image not found in DOM' }, context: PAGE })
  appAnswers(201)

  await captureAndDeliver(CLICKED)

  expect(postedMeta().adapter).toEqual(RECORD)
  expect(requested).toContain(CLICKED.imageUrl)
})

it('delivers the capture with no record when the tab answers nothing', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: undefined })
  appAnswers(201)

  await captureAndDeliver(CLICKED)

  expect(postedMeta().adapter).toBeUndefined()
  // No content script, so nothing better than what the tab record says.
  expect(postedMeta().pageTitle).toBe(CLICKED.pageTitle)
  expect((await readHistory())[0]!.status).toBe('delivered')
})

// The tab's title is what chrome last saw; on a single-page app that is often
// the screen before this one.
it('titles the capture the way the page does, not the way the tab does', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: PAGE })
  appAnswers(201)

  await captureAndDeliver({ ...CLICKED, pageTitle: '首頁 / X' })

  expect(postedMeta().pageTitle).toBe('Alice (@alice) on X: hello')
})

it('keeps the tab title when the page could not name itself', async () => {
  tabAnswers({
    capture: { dataUrl: DATA_URL, width: 1, height: 1 },
    context: { record: RECORD, pageTitle: '' },
  })
  appAnswers(201)

  await captureAndDeliver(CLICKED)

  expect(postedMeta().pageTitle).toBe(CLICKED.pageTitle)
})

it('keeps the bytes, badges the failure and notifies once when the app is closed', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: PAGE })
  appAnswers(0)

  await captureAndDeliver(CLICKED)

  const [row] = await readHistory()
  expect(row).toMatchObject({ status: 'failed', hasBytes: true })
  expect(row!.reason).toBe('Could not reach the app')
  expect(chrome.badgeText()).toBe('1')
  expect(chrome.notifications.create).toHaveBeenCalledTimes(1)
  expect(chrome.notifications.create.mock.calls[0]![1]).toMatchObject({
    title: 'BooruBox',
    message: 'Open BooruBox to receive this image',
  })
})

it('retries the kept bytes without asking the image host again', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: PAGE })
  appAnswers(0)
  await captureAndDeliver(CLICKED)
  const [failed] = await readHistory()
  requested = []
  appAnswers(200)

  await retryCapture(failed!.id)

  const [row] = await readHistory()
  expect(row).toMatchObject({ id: failed!.id, status: 'delivered', hasBytes: false })
  expect(row!.reason).toBeUndefined()
  expect(await getBytes(failed!.id)).toBeUndefined()
  expect(chrome.badgeText()).toBe('')
  // Only the POST: the image host is not asked again, and nothing is announced
  // either — the bytes are in hand (spec `capture-delivery`, "Retry").
  expect(requested).toEqual([CAPTURES])
  expect(postedMeta().id).toBe(failed!.id)
})

it('withdraws the announcement and makes no entry when neither route produced bytes', async () => {
  tabAnswers({ capture: { error: 'Image not found in DOM' } })
  fetchMock.mockImplementation(async (url: string) => {
    record(url)
    return new Response('', { status: 403, statusText: 'Forbidden' })
  })

  await captureAndDeliver(CLICKED)

  const withdrawals = fetchedFor(`${PENDING}/`)
  expect(withdrawals).toHaveLength(1)
  const [url, init] = withdrawals[0]!
  expect(String(url)).toBe(`${PENDING}/${announcedMeta().id}`)
  expect(init.method).toBe('DELETE')
  expect(JSON.parse(init.body as string)).toEqual({ reason: 'HTTP 403: Forbidden' })
  expect(await readHistory()).toEqual([])
  expect(chrome.notifications.create).toHaveBeenCalledTimes(1)
  expect(chrome.badgeText()).toBe('')
})

it('says so rather than refetching when a failed entry has lost its bytes', async () => {
  tabAnswers({ capture: { dataUrl: DATA_URL, width: 1, height: 1 }, context: PAGE })
  appAnswers(0)
  await captureAndDeliver(CLICKED)
  const [failed] = await readHistory()
  indexedDB.deleteDatabase('boorubox-bridge')
  requested = []

  await retryCapture(failed!.id)

  expect((await readHistory())[0]!.reason).toBe('The captured bytes are no longer in the browser')
  expect(requested).toEqual([])
})
