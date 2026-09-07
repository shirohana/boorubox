import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import type { CaptureMeta } from '@boorubox/shared'

import { postCapture, probeStatus } from './client.js'

const CAPTURES = 'http://127.0.0.1:47201/captures'
const STATUS = 'http://127.0.0.1:47201/status'

const META: CaptureMeta = {
  id: 'e5b1c2b4-0000-4000-8000-000000000001',
  imageUrl: 'https://cdn.test/a.png',
  pageUrl: 'https://page.test/gallery',
  pageTitle: 'a page',
  capturedAt: 1_700_000_000_000,
}

const BLOB = new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' })

let fetchMock: ReturnType<typeof vi.fn>

function json(body: unknown, status: number) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function abortError(name: 'AbortError' | 'TimeoutError') {
  const error = new Error('aborted')
  error.name = name
  return error
}

beforeEach(() => {
  fetchMock = vi.fn()
  vi.stubGlobal('fetch', fetchMock)
})

afterEach(() => {
  vi.unstubAllGlobals()
})

it('sends the bytes and the meta under the names the app requires', async () => {
  fetchMock.mockResolvedValue(json({ id: META.id }, 201))

  await postCapture(CAPTURES, META, BLOB)

  const [url, init] = fetchMock.mock.calls[0]!
  expect(url).toBe(CAPTURES)
  expect(init.method).toBe('POST')
  const body = init.body as FormData
  expect(body.get('file')).toBeInstanceOf(Blob)
  expect(JSON.parse(body.get('meta') as string)).toEqual(META)
  // The boundary has to come from the form, so nothing may set the type here.
  expect(init.headers).toBeUndefined()
})

it('counts a stored capture as delivered', async () => {
  fetchMock.mockResolvedValue(json({ id: META.id }, 201))

  expect(await postCapture(CAPTURES, META, BLOB)).toEqual({ delivered: true })
})

it('counts a capture the app already held as delivered', async () => {
  fetchMock.mockResolvedValue(json({ id: META.id }, 200))

  expect(await postCapture(CAPTURES, META, BLOB)).toEqual({ delivered: true })
})

it('fails with the reason the app gave when it refuses', async () => {
  fetchMock.mockResolvedValue(json({ error: 'the "file" part is missing' }, 400))

  expect(await postCapture(CAPTURES, META, BLOB)).toEqual({
    delivered: false,
    reason: 'the "file" part is missing',
  })
})

it('fails with the status line when a refusal carries no reason', async () => {
  fetchMock.mockResolvedValue(new Response('nope', { status: 500, statusText: 'Server Error' }))

  expect(await postCapture(CAPTURES, META, BLOB)).toEqual({
    delivered: false,
    reason: 'HTTP 500 Server Error',
  })
})

it('fails when the app cannot be reached at all', async () => {
  fetchMock.mockRejectedValue(new TypeError('Failed to fetch'))

  expect(await postCapture(CAPTURES, META, BLOB)).toEqual({
    delivered: false,
    reason: 'Could not reach the app',
  })
})

it('fails when the request is aborted or times out', async () => {
  fetchMock.mockRejectedValue(abortError('TimeoutError'))
  expect(await postCapture(CAPTURES, META, BLOB)).toEqual({
    delivered: false,
    reason: 'The app did not answer within 30 seconds',
  })

  fetchMock.mockRejectedValue(abortError('AbortError'))
  expect(await postCapture(CAPTURES, META, BLOB)).toEqual({
    delivered: false,
    reason: 'The app did not answer within 30 seconds',
  })
})

it('reports a running app with its library and image count', async () => {
  fetchMock.mockResolvedValue(json(
    { version: '0.1.0', libraryPath: '/Users/hana/Pictures/BooruBox', imageCount: 42 },
    200,
  ))

  expect(await probeStatus(STATUS)).toEqual({
    state: 'connected',
    imageCount: 42,
    libraryPath: '/Users/hana/Pictures/BooruBox',
  })
})

it('tells a running app with no library from one that is not running', async () => {
  fetchMock.mockResolvedValue(json({ error: 'no library is open' }, 503))
  expect(await probeStatus(STATUS)).toEqual({ state: 'no-library' })

  fetchMock.mockRejectedValue(new TypeError('Failed to fetch'))
  expect(await probeStatus(STATUS)).toEqual({ state: 'unreachable' })
})

it('treats a 200 that is not a status as nothing to talk to', async () => {
  fetchMock.mockResolvedValue(new Response('<html>a captive portal</html>', { status: 200 }))

  expect(await probeStatus(STATUS)).toEqual({ state: 'unreachable' })
})
