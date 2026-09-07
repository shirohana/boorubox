import 'fake-indexeddb/auto'

import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { createEntry, type CaptureEntry } from '../delivery/state.js'
import { getBytes, putBytes } from './blobs.js'
import { installFakeChrome, type FakeChrome } from '../test-support/chrome.js'
import {
  beginCapture,
  clearDelivered,
  DELIVERED_CAP,
  discardEntry,
  ENTRY_PREFIX,
  readEntry,
  readHistory,
  recordDelivered,
  recordFailed,
  sweepInterrupted,
  writeEntry,
} from './store.js'

let chrome: FakeChrome

function entry(id: string, overrides: Partial<CaptureEntry> = {}): CaptureEntry {
  return {
    ...createEntry({
      id,
      imageUrl: `https://cdn.test/${id}.png`,
      pageUrl: 'https://page.test/gallery',
      pageTitle: 'a page',
      capturedAt: 1_700_000_000_000,
      size: 3,
      thumbnail: 'data:image/jpeg;base64,AAAA',
    }),
    ...overrides,
  }
}

function bytes() {
  return new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' })
}

beforeEach(async () => {
  chrome = installFakeChrome()
  indexedDB.deleteDatabase('boorubox-bridge')
})

afterEach(() => {
  vi.unstubAllGlobals()
})

it('keeps one key per capture, and the entry survives a read back', async () => {
  await beginCapture(entry('a'), bytes())

  expect([...chrome.storage.local.data.keys()]).toEqual([`${ENTRY_PREFIX}a`])
  expect(await readEntry('a')).toMatchObject({ id: 'a', status: 'pending', size: 3 })
  expect(await getBytes('a')).toBeInstanceOf(Blob)
})

it('shows a row as retryable only while its bytes are there', async () => {
  await beginCapture(entry('a'), bytes())
  await recordFailed(entry('a'), 'Could not reach the app')

  const [row] = await readHistory()

  expect(row).toMatchObject({ id: 'a', status: 'failed', hasBytes: true })
})

it('releases the bytes once the app has taken the capture', async () => {
  await beginCapture(entry('a'), bytes())

  await recordDelivered(entry('a'))

  expect(await getBytes('a')).toBeUndefined()
  expect(await readEntry('a')).toMatchObject({ status: 'delivered' })
  expect((await readHistory())[0]!.hasBytes).toBe(false)
})

it('counts failures on the badge and nothing else', async () => {
  await beginCapture(entry('a'), bytes())
  await beginCapture(entry('b', { capturedAt: 1_700_000_000_001 }), bytes())
  expect(chrome.badgeText()).toBe('')

  await recordFailed(entry('a'), 'Could not reach the app')
  await recordFailed(entry('b'), 'Could not reach the app')
  expect(chrome.badgeText()).toBe('2')

  await recordDelivered(await readEntry('a') as CaptureEntry)
  expect(chrome.badgeText()).toBe('1')

  await recordDelivered(await readEntry('b') as CaptureEntry)
  expect(chrome.badgeText()).toBe('')
})

it('clears the delivered entries and leaves every failure with its bytes', async () => {
  await beginCapture(entry('kept', { capturedAt: 2 }), bytes())
  await recordFailed(entry('kept', { capturedAt: 2 }), 'Could not reach the app')
  await beginCapture(entry('gone', { capturedAt: 1 }), bytes())
  await recordDelivered(entry('gone', { capturedAt: 1 }))

  await clearDelivered()

  const rows = await readHistory()
  expect(rows.map((row) => row.id)).toEqual(['kept'])
  expect(rows[0]!.hasBytes).toBe(true)
  expect(chrome.badgeText()).toBe('1')
})

it('discards one failed entry with its bytes and touches no other', async () => {
  await beginCapture(entry('a', { capturedAt: 2 }), bytes())
  await recordFailed(entry('a', { capturedAt: 2 }), 'Could not reach the app')
  await beginCapture(entry('b', { capturedAt: 1 }), bytes())
  await recordFailed(entry('b', { capturedAt: 1 }), 'Could not reach the app')

  await discardEntry('a')

  expect(await getBytes('a')).toBeUndefined()
  expect(await getBytes('b')).toBeInstanceOf(Blob)
  expect((await readHistory()).map((row) => row.id)).toEqual(['b'])
  expect(chrome.badgeText()).toBe('1')
})

it('fails everything left in flight when the worker starts again', async () => {
  await writeEntry(entry('a', { capturedAt: 3 }))
  await writeEntry(entry('b', { capturedAt: 2, status: 'delivered' }))
  await writeEntry(entry('c', { capturedAt: 1 }))

  const swept = await sweepInterrupted()

  expect(swept.map((e) => e.id)).toEqual(['a', 'c'])
  expect((await readEntry('a'))?.status).toBe('failed')
  expect((await readEntry('b'))?.status).toBe('delivered')
  expect(chrome.badgeText()).toBe('2')
})

it('caps delivered entries without ever dropping a failure', async () => {
  await putBytes('failed-old', bytes())
  await writeEntry(entry('failed-old', { capturedAt: 0, status: 'failed', reason: 'old' }))
  for (let index = 0; index < DELIVERED_CAP + 5; index += 1) {
    await writeEntry(entry(`d${index}`, { capturedAt: 1000 + index, status: 'delivered' }))
  }

  // The trim runs on the delivery that tips it over the cap.
  await recordDelivered(entry('newest', { capturedAt: 9_999 }))

  const rows = await readHistory()
  const delivered = rows.filter((row) => row.status === 'delivered')
  expect(delivered).toHaveLength(DELIVERED_CAP)
  expect(delivered[0]!.id).toBe('newest')
  expect(rows.some((row) => row.id === 'failed-old')).toBe(true)
  expect(await getBytes('failed-old')).toBeInstanceOf(Blob)
})
