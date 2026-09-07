import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { installFakeChrome, type FakeChrome } from '../test-support/chrome.js'

const IMAGE_URL = 'https://cdn.test/a.png'
const PAGE_URL = 'https://page.test/gallery'
// A one-pixel PNG as `canvas.toDataURL()` would hand it over.
const DATA_URL = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0l'
  + 'EQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='

let chrome: FakeChrome
let fetchMock: ReturnType<typeof vi.fn>

/** Fresh module per test: the rule-id counter is module state. */
async function loadCapture() {
  vi.resetModules()
  return import('./capture.js')
}

beforeEach(() => {
  chrome = installFakeChrome()
  fetchMock = vi.fn(async () => {
    chrome.events.push('fetch')
    return new Response(new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' }))
  })
  vi.stubGlobal('fetch', fetchMock)
})

afterEach(() => {
  vi.unstubAllGlobals()
})

it('takes the bytes from the page when the tab answers', async () => {
  chrome.tabs.sendMessage.mockResolvedValue({ dataUrl: DATA_URL, width: 1, height: 1 })
  const { captureImageBytes } = await loadCapture()

  const blob = await captureImageBytes(1, IMAGE_URL, PAGE_URL)

  expect(blob.type).toBe('image/png')
  expect(blob.size).toBeGreaterThan(0)
  expect(fetchMock).not.toHaveBeenCalled()
  expect(chrome.declarativeNetRequest.updateDynamicRules).not.toHaveBeenCalled()
})

it('falls back to fetching when the page cannot produce the bytes', async () => {
  chrome.tabs.sendMessage.mockResolvedValue({ error: 'Image not found in DOM' })
  const { captureImageBytes } = await loadCapture()

  const blob = await captureImageBytes(1, IMAGE_URL, PAGE_URL)

  expect(blob.size).toBe(3)
  expect(fetchMock).toHaveBeenCalledWith(IMAGE_URL)
})

it('falls back to fetching when no content script answers at all', async () => {
  chrome.tabs.sendMessage.mockRejectedValue(new Error('Receiving end does not exist'))
  const { captureImageBytes } = await loadCapture()

  await captureImageBytes(1, IMAGE_URL, PAGE_URL)

  expect(fetchMock).toHaveBeenCalledWith(IMAGE_URL)
})

it('sets Referer to the page for the image host, over xmlhttprequest only', async () => {
  chrome.tabs.sendMessage.mockResolvedValue({ error: 'no' })
  const { captureImageBytes } = await loadCapture()

  await captureImageBytes(1, IMAGE_URL, PAGE_URL)

  const { addRules } = chrome.declarativeNetRequest.updateDynamicRules.mock.calls[0]![0]
  expect(addRules[0].action).toEqual({
    type: 'modifyHeaders',
    requestHeaders: [{ header: 'Referer', operation: 'set', value: PAGE_URL }],
  })
  expect(addRules[0].condition).toEqual({
    urlFilter: 'cdn.test',
    resourceTypes: ['xmlhttprequest'],
  })
})

it('has the rule in place before the fetch and gone after it', async () => {
  chrome.tabs.sendMessage.mockResolvedValue({ error: 'no' })
  const { captureImageBytes } = await loadCapture()

  await captureImageBytes(1, IMAGE_URL, PAGE_URL)

  expect(chrome.events).toEqual(['rule.add:1', 'fetch', 'rule.remove:1'])
  expect(chrome.declarativeNetRequest.rules.size).toBe(0)
})

it('removes the rule even when the fetch fails', async () => {
  chrome.tabs.sendMessage.mockResolvedValue({ error: 'no' })
  fetchMock.mockResolvedValue(new Response('', { status: 403, statusText: 'Forbidden' }))
  const { captureImageBytes } = await loadCapture()

  await expect(captureImageBytes(1, IMAGE_URL, PAGE_URL)).rejects.toThrow('HTTP 403')

  expect(chrome.declarativeNetRequest.rules.size).toBe(0)
  expect(chrome.events).toContain('rule.remove:1')
})

it('gives two captures started in the same second different rule ids', async () => {
  chrome.tabs.sendMessage.mockResolvedValue({ error: 'no' })
  const { captureImageBytes } = await loadCapture()
  // Both rules exist at once: the fake refuses a colliding id, which is what
  // Chrome does and what the legacy clock-derived id walked into.
  let releaseFirst = () => {}
  fetchMock.mockImplementationOnce(async () => {
    await new Promise<void>((resolve) => {
      releaseFirst = resolve
    })
    return new Response(new Blob(['a']))
  })

  const first = captureImageBytes(1, IMAGE_URL, PAGE_URL)
  const second = captureImageBytes(2, IMAGE_URL, PAGE_URL)
  await second
  releaseFirst()
  await first

  const ids = chrome.declarativeNetRequest.updateDynamicRules.mock.calls
    .flatMap(([options]) => options.addRules ?? [])
    .map((rule: { id: number }) => rule.id)
  expect(new Set(ids).size).toBe(ids.length)
  expect(ids).toEqual([1, 2])
})
