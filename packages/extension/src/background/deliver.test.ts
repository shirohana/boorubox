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

let chrome: FakeChrome
let fetchMock: ReturnType<typeof vi.fn>
/** Every URL fetched, so a test can say the image host was never asked. */
let requested: string[]

/** The tab answers the canvas capture and the context ask independently. */
function tabAnswers(options: { capture?: unknown, context?: unknown }) {
  chrome.tabs.sendMessage.mockImplementation(async (_id: number, message: { type: string }) => {
    if (message.type === CAPTURE_IMAGE) return options.capture
    if (message.type === EXTRACT_CONTEXT) return options.context
    return undefined
  })
}

function appAnswers(status: number) {
  fetchMock.mockImplementation(async (url: string) => {
    requested.push(url)
    if (url.startsWith('http://127.0.0.1')) {
      if (status === 0) throw new TypeError('Failed to fetch')
      return new Response(JSON.stringify({ id: 'stored' }), { status })
    }
    return new Response(new Blob([new Uint8Array([9, 9, 9])], { type: 'image/png' }))
  })
}

function postedMeta() {
  const call = fetchMock.mock.calls.find(([url]) => String(url).startsWith('http://127.0.0.1'))!
  const body = call[1].body as FormData
  return JSON.parse(body.get('meta') as string)
}

beforeEach(() => {
  chrome = installFakeChrome()
  requested = []
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
  expect(requested).toEqual(['http://127.0.0.1:47201/captures'])
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
  expect(requested).toEqual(['http://127.0.0.1:47201/captures'])
  expect(postedMeta().id).toBe(failed!.id)
})

it('makes no entry at all when neither route produced bytes', async () => {
  tabAnswers({ capture: { error: 'Image not found in DOM' } })
  fetchMock.mockImplementation(async (url: string) => {
    requested.push(url)
    return new Response('', { status: 403, statusText: 'Forbidden' })
  })

  await captureAndDeliver(CLICKED)

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
