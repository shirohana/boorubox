// @vitest-environment jsdom

import 'fake-indexeddb/auto'

import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { renderConnection } from './connection.js'
import { renderHistory } from './history-view.js'
import { mountPopup } from './main.js'
import type { HistoryEntry } from '../history/store.js'
import { beginCapture, ENTRY_PREFIX, recordFailed } from '../history/store.js'
import { createEntry } from '../delivery/state.js'
import { installFakeChrome, type FakeChrome } from '../test-support/chrome.js'

let chrome: FakeChrome
let unmount: (() => void) | undefined

function row(id: string, overrides: Partial<HistoryEntry> = {}): HistoryEntry {
  return {
    ...createEntry({
      id,
      imageUrl: `https://cdn.test/${id}.png`,
      pageUrl: 'https://x.com/alice/status/1',
      pageTitle: 'alice on X',
      capturedAt: 1_700_000_000_000,
      size: 2048,
      thumbnail: 'data:image/jpeg;base64,AAAA',
    }),
    hasBytes: true,
    ...overrides,
  }
}

const SHELL = `
  <div class="panel">
    <div id="connection"></div>
    <div class="toolbar">
      <button id="retry-all" type="button" hidden></button>
      <button id="clear" type="button" hidden></button>
    </div>
    <div id="history"></div>
    <footer>
      <input id="port" type="number" />
      <span id="port-error" hidden></span>
    </footer>
  </div>
`

/**
 * Waits for the popup's own reads to reach the DOM, and answers what they drew.
 *
 * A fixed number of turns is not enough here: a mount reads `chrome.storage`
 * and IndexedDB, and how many turns those take depends on how busy the machine
 * is — under the whole workspace's test run, more than under this file alone.
 */
async function until<T>(what: string, drawn: () => T | null | undefined | false): Promise<T> {
  for (let turn = 0; turn < 500; turn += 1) {
    const value = drawn()
    if (value) {
      return value
    }
    await new Promise((resolve) => setTimeout(resolve, 1))
  }
  throw new Error(`the popup never ${what}`)
}

function historyRow() {
  return document.querySelector('#history .row')
}

beforeEach(() => {
  chrome = installFakeChrome()
  indexedDB.deleteDatabase('boorubox-bridge')
  document.body.innerHTML = SHELL
})

afterEach(() => {
  unmount?.()
  unmount = undefined
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

it('says which of the three things the app is doing', () => {
  const connected = renderConnection({
    state: 'connected',
    imageCount: 1234,
    libraryPath: '/Users/hana/Pictures/BooruBox',
  })
  expect(connected.dataset.state).toBe('connected')
  expect(connected.textContent).toContain('1,234 images')

  const noLibrary = renderConnection({ state: 'no-library' })
  expect(noLibrary.dataset.state).toBe('no-library')
  expect(noLibrary.textContent).toContain('no library is open')

  const unreachable = renderConnection({ state: 'unreachable' })
  expect(unreachable.dataset.state).toBe('unreachable')
  expect(unreachable.textContent).toContain('not running')
})

it('draws a row per capture with what it is and what became of it', () => {
  const list = renderHistory([
    row('a', { status: 'delivered', hasBytes: false }),
    row('b', { status: 'failed', reason: 'Could not reach the app' }),
    row('c', { status: 'pending' }),
    row('d', { status: 'failed', reason: 'Could not reach the app', hasBytes: false }),
  ])

  const rows = [...list.querySelectorAll('.row')]
  expect(rows.map((element) => (element as HTMLElement).dataset.id)).toEqual(['a', 'b', 'c', 'd'])

  const first = rows[0]!
  expect(first.querySelector('img.thumb')?.getAttribute('src')).toBe('data:image/jpeg;base64,AAAA')
  expect(first.querySelector('.title')?.textContent).toBe('alice on X')
  expect(first.querySelector('.source')?.textContent).toBe('https://cdn.test/a.png')
  expect(first.querySelector('.meta')?.textContent).toContain('2 KB')
  expect(first.querySelector('.meta')?.textContent).toContain('Saved')
  expect(first.querySelectorAll('button')).toHaveLength(0)

  const failed = rows[1]!
  expect(failed.querySelector('.meta')?.textContent).toContain('Could not reach the app')
  expect([...failed.querySelectorAll('button')].map((b) => b.textContent))
    .toEqual(['Retry', 'Discard'])

  expect(rows[2]!.querySelector('.meta')?.textContent).toContain('Sending…')

  // Bytes gone: discard is the only thing left, and the row says why.
  const lost = rows[3]!
  expect([...lost.querySelectorAll('button')].map((b) => b.textContent)).toEqual(['Discard'])
  expect(lost.querySelector('.meta')?.textContent).toContain('discard only')
})

it('says so when there is nothing captured yet', () => {
  expect(renderHistory([]).querySelector('.empty')?.textContent).toContain('No captures yet')
})

it('redraws the list when the worker changes an entry, without reopening', async () => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response('', { status: 503 })))
  unmount = mountPopup(document)
  await until('drew the empty list', () => document.querySelector('#history .empty'))

  const bytes = new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' })
  await beginCapture(row('a'), bytes)
  await until('redrew the row as pending',
    () => historyRow()?.getAttribute('data-status') === 'pending')

  await recordFailed(row('a'), 'Could not reach the app')
  await until('redrew the row as failed',
    () => historyRow()?.getAttribute('data-status') === 'failed')

  expect(document.querySelector<HTMLElement>('#retry-all')?.hidden).toBe(false)
  expect([...chrome.storage.onChanged.listeners]).toHaveLength(1)
  expect([...chrome.storage.local.data.keys()]).toContain(`${ENTRY_PREFIX}a`)
})

it('asks the worker to retry rather than posting from the popup', async () => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response('', { status: 503 })))
  const bytes = new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' })
  await beginCapture(row('a'), bytes)
  await recordFailed(row('a'), 'Could not reach the app')
  unmount = mountPopup(document)
  const retry = await until('drew the failed row\'s Retry',
    () => document.querySelector<HTMLButtonElement>('button[data-action="retry"]'))

  retry.click()

  expect(chrome.runtime.sendMessage).toHaveBeenCalledWith({ type: 'RETRY_CAPTURE', id: 'a' })
})

it('refuses a port the app could never be listening on, and keeps the old one', async () => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response('', { status: 503 })))
  unmount = mountPopup(document)
  await until('drew the empty list', () => document.querySelector('#history .empty'))

  const port = document.querySelector<HTMLInputElement>('#port')!
  port.value = '70000'
  port.dispatchEvent(new Event('change'))
  await until('showed the port error',
    () => !document.querySelector<HTMLElement>('#port-error')!.hidden)

  expect(chrome.storage.local.data.has('port')).toBe(false)
})
