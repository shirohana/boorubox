// @vitest-environment jsdom

import type { RebuildReport } from '@boorubox/shared'
import { emit } from '@tauri-apps/api/event'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { REBUILD_PROGRESS_EVENT } from './events'
import { Rebuild } from './rebuild.svelte'

afterEach(() => {
  clearMocks()
})

function report(overrides: Partial<RebuildReport> = {}): RebuildReport {
  return {
    images: 3,
    failed: 0,
    failures: [],
    keptAs: 'library.sqlite.corrupt-1700000000000',
    rules: 1,
    sites: 0,
    ...overrides,
  }
}

it('tracks progress while the command is in flight and lands the report', async () => {
  let settle: ((value: RebuildReport) => void) | null = null
  mockIPC((cmd) => {
    if (cmd === 'rebuild_library') {
      return new Promise<RebuildReport>((resolve) => {
        settle = resolve
      })
    }
    return null
  }, { shouldMockEvents: true })

  const rebuild = new Rebuild()
  const running = rebuild.run('/library')

  expect(rebuild.running).toBe(true)
  expect(rebuild.report).toBeNull()

  // Waits until `rebuild_library` has actually been invoked — `run` awaits
  // `onRebuildProgress`'s own subscription first, so `settle` is not set the
  // moment `run` is called.
  await vi.waitFor(() => expect(settle).not.toBeNull())

  await emit(REBUILD_PROGRESS_EVENT, { done: 1, total: 3 })
  expect(rebuild.progress).toEqual({ done: 1, total: 3 })

  settle!(report())
  await expect(running).resolves.toEqual(report())

  expect(rebuild.running).toBe(false)
  expect(rebuild.report).toEqual(report())
  expect(rebuild.error).toBeNull()
})

it('records the error and answers null on a rejected command', async () => {
  mockIPC(() => {
    throw 'that library is damaged'
  }, { shouldMockEvents: true })

  const rebuild = new Rebuild()
  const outcome = await rebuild.run('/library')

  expect(outcome).toBeNull()
  expect(rebuild.running).toBe(false)
  expect(rebuild.report).toBeNull()
  expect(rebuild.error).toBe('that library is damaged')
})

it('drops its progress subscription when the run ends', async () => {
  mockIPC(() => report(), { shouldMockEvents: true })

  const rebuild = new Rebuild()
  await rebuild.run('/library')

  // A tick after the command answered — the rebuild that emitted it is over,
  // and a listener left behind would make every later rebuild count its
  // progress once per run that came before it.
  await emit(REBUILD_PROGRESS_EVENT, { done: 7, total: 9 })

  expect(rebuild.progress).toBeNull()
})

it('reset clears the report and the error for a second run', async () => {
  mockIPC(() => report(), { shouldMockEvents: true })

  const rebuild = new Rebuild()
  await rebuild.run('/library')
  expect(rebuild.report).not.toBeNull()

  rebuild.reset()

  expect(rebuild.report).toBeNull()
  expect(rebuild.error).toBeNull()
})
