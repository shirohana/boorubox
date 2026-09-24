// @vitest-environment jsdom

import type { ThumbsReport } from '@boorubox/shared'
import { emit } from '@tauri-apps/api/event'
import { clearMocks, mockConvertFileSrc, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { cachedThumbnail, thumbnail } from '$lib/components/library/thumbnail-cache.svelte'
import { THUMBS_PROGRESS_EVENT } from './events'
import { ThumbsRegenerate } from './thumbs-regenerate.svelte'

afterEach(() => {
  clearMocks()
})

function report(overrides: Partial<ThumbsReport> = {}): ThumbsReport {
  return { regenerated: 300, failed: 0, ...overrides }
}

it('tracks progress while the command is in flight and lands the report', async () => {
  let settle: ((value: ThumbsReport) => void) | null = null
  mockIPC((cmd) => {
    if (cmd === 'regenerate_thumbnails') {
      return new Promise<ThumbsReport>((resolve) => {
        settle = resolve
      })
    }
    return null
  }, { shouldMockEvents: true })

  const store = new ThumbsRegenerate()
  await store.subscribe()
  const running = store.start()

  expect(store.running).toBe(true)
  expect(store.report).toBeNull()

  // Waits until `regenerate_thumbnails` has actually been invoked, the same
  // reason `rebuild.svelte.test.ts` waits before its first tick.
  await vi.waitFor(() => expect(settle).not.toBeNull())

  await emit(THUMBS_PROGRESS_EVENT, { done: 100, total: 300 })
  expect(store.progress).toEqual({ done: 100, total: 300 })

  settle!(report())
  await expect(running).resolves.toEqual(report())

  expect(store.running).toBe(false)
  expect(store.report).toEqual(report())
  expect(store.error).toBeNull()
})

it('records the error and answers null on a rejected command', async () => {
  mockIPC(() => {
    throw 'thumbnails are already being regenerated'
  }, { shouldMockEvents: true })

  const store = new ThumbsRegenerate()
  await store.subscribe()
  const outcome = await store.start()

  expect(outcome).toBeNull()
  expect(store.running).toBe(false)
  expect(store.report).toBeNull()
  expect(store.error).toBe('thumbnails are already being regenerated')
})

it('reset clears the progress and the report for a second run', async () => {
  mockIPC(() => report(), { shouldMockEvents: true })

  const store = new ThumbsRegenerate()
  await store.subscribe()
  await store.start()
  await emit(THUMBS_PROGRESS_EVENT, { done: 300, total: 300 })
  expect(store.report).not.toBeNull()

  store.reset()

  expect(store.progress).toBeNull()
  expect(store.report).toBeNull()
  expect(store.error).toBeNull()
})

it('drops the thumbnail cache once the last tick arrives', async () => {
  mockConvertFileSrc('macos')
  mockIPC((cmd, args) => {
    if (cmd === 'thumbnail_path') {
      return { path: `/library/.thumbs/${(args as { id: string }).id}.jpg`, version: 1 }
    }
    // `regenerate_thumbnails` never settles in this test: the drop under test
    // is the one the last tick causes, not the one `start` makes on its own
    // resolution (covered below).
    return new Promise<ThumbsReport>(() => {})
  }, { shouldMockEvents: true })

  await thumbnail('abc')
  expect(cachedThumbnail('abc')).not.toBeNull()

  const store = new ThumbsRegenerate()
  await store.subscribe()
  void store.start()

  await emit(THUMBS_PROGRESS_EVENT, { done: 300, total: 300 })

  expect(cachedThumbnail('abc')).toBeNull()
})

it('drops the thumbnail cache when the run resolves, even with no ticks', async () => {
  mockConvertFileSrc('macos')
  mockIPC((cmd, args) => {
    if (cmd === 'thumbnail_path') {
      return { path: `/library/.thumbs/${(args as { id: string }).id}.jpg`, version: 1 }
    }
    if (cmd === 'regenerate_thumbnails') return report({ regenerated: 0 })
    return null
  }, { shouldMockEvents: true })

  await thumbnail('xyz')
  expect(cachedThumbnail('xyz')).not.toBeNull()

  const store = new ThumbsRegenerate()
  await store.subscribe()
  await store.start()

  expect(cachedThumbnail('xyz')).toBeNull()
})

it('drops a report from a run reset() has already superseded', async () => {
  let settle: ((value: ThumbsReport) => void) | null = null
  mockIPC((cmd) => {
    if (cmd === 'regenerate_thumbnails') {
      return new Promise<ThumbsReport>((resolve) => {
        settle = resolve
      })
    }
    return null
  }, { shouldMockEvents: true })

  const store = new ThumbsRegenerate()
  await store.subscribe()
  const running = store.start()
  await vi.waitFor(() => expect(settle).not.toBeNull())

  // A library switch, mid-pass: the stopped pass's command is still
  // unwinding when the screen it was writing for goes away.
  store.reset()

  settle!(report())
  await expect(running).resolves.toEqual(report())

  expect(store.report).toBeNull()
})

it('drops a tick from a run reset() has already superseded', async () => {
  mockIPC((cmd) => {
    // Never settles: the tick under test arrives while the command is still
    // in flight, which is exactly the window a library switch falls in.
    if (cmd === 'regenerate_thumbnails') return new Promise<ThumbsReport>(() => {})
    return null
  }, { shouldMockEvents: true })

  const store = new ThumbsRegenerate()
  await store.subscribe()
  void store.start()

  store.reset()

  await emit(THUMBS_PROGRESS_EVENT, { done: 10, total: 20 })

  expect(store.progress).toBeNull()
})

it('start() returns early while a pass is already running, leaving it untouched', async () => {
  let settle: ((value: ThumbsReport) => void) | null = null
  let calls = 0
  mockIPC((cmd) => {
    if (cmd === 'regenerate_thumbnails') {
      calls++
      return new Promise<ThumbsReport>((resolve) => {
        settle = resolve
      })
    }
    return null
  }, { shouldMockEvents: true })

  const store = new ThumbsRegenerate()
  await store.subscribe()
  const first = store.start()
  await vi.waitFor(() => expect(calls).toBe(1))

  await emit(THUMBS_PROGRESS_EVENT, { done: 5, total: 10 })
  expect(store.progress).toEqual({ done: 5, total: 10 })

  const second = await store.start()

  expect(second).toBeNull()
  expect(calls).toBe(1)
  expect(store.running).toBe(true)
  expect(store.progress).toEqual({ done: 5, total: 10 })

  settle!(report())
  await first
})
