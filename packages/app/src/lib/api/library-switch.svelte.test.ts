// @vitest-environment jsdom
// `imports` is exercised for real (not stubbed) via its own IPC mock, so
// `guard` is tested against the actual queue it reads `runs` off of.

import type { ImportReport } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { Imports } from './imports.svelte'
import { LibrarySwitch } from './library-switch.svelte'

afterEach(() => {
  clearMocks()
})

function cancelled(imported: number): ImportReport {
  return { imported, skipped: 0, failed: 0, cancelled: true, items: [] }
}

/** Holds `import_bundle` open until the test settles it. */
function stubbedImports() {
  const calls: { settle: (report: ImportReport) => void }[] = []
  const cancelCalls = vi.fn()
  mockIPC((cmd) => {
    if (cmd === 'import_bundle') {
      return new Promise<ImportReport>((settle) => {
        calls.push({ settle })
      })
    }
    if (cmd === 'library_status') return { opened: true, libraryPath: '/library' }
    if (cmd === 'import_cancel') return cancelCalls()
    throw `unexpected command ${cmd}`
  }, { shouldMockEvents: true })
  return {
    calls,
    cancelCalls,
    started: (n: number) => vi.waitFor(() => expect(calls).toHaveLength(n)),
  }
}

it('runs the action at once with nothing running or queued, asking no question', async () => {
  const switcher = new LibrarySwitch(new Imports())
  const action = vi.fn().mockResolvedValue(undefined)

  await switcher.guard('switch', action)

  expect(action).toHaveBeenCalledTimes(1)
  expect(switcher.pending).toBeNull()
})

it('parks the action behind a question while an import is running', async () => {
  const stub = stubbedImports()
  const imports = new Imports()
  const switcher = new LibrarySwitch(imports)
  const action = vi.fn().mockResolvedValue(undefined)

  imports.enqueueBundle(['/a.db'])
  await stub.started(1)

  const guarding = switcher.guard('switch', action)
  await vi.waitFor(() => expect(switcher.pending).not.toBeNull())
  expect(switcher.pending).toMatchObject({ kind: 'bundle', queued: 0, action: 'switch' })
  expect(action).not.toHaveBeenCalled()

  switcher.decline()
  await guarding
})

it('declining never runs the action and leaves the queue as it was', async () => {
  const stub = stubbedImports()
  const imports = new Imports()
  const switcher = new LibrarySwitch(imports)
  const action = vi.fn().mockResolvedValue(undefined)

  imports.enqueueBundle(['/a.db'])
  await stub.started(1)

  await Promise.all([
    switcher.guard('close', action),
    vi.waitFor(() => expect(switcher.pending).not.toBeNull()).then(() => switcher.decline()),
  ])

  expect(action).not.toHaveBeenCalled()
  expect(switcher.pending).toBeNull()
  expect(imports.runs).toHaveLength(1)

  stub.calls[0].settle(cancelled(0))
})

it('confirming cancels, empties the queue, and only then runs the action', async () => {
  const stub = stubbedImports()
  const imports = new Imports()
  const switcher = new LibrarySwitch(imports)
  const order: string[] = []
  const action = vi.fn(async () => {
    order.push('action')
  })

  imports.enqueueBundle(['/a.db'])
  await stub.started(1)

  const guarding = switcher.guard('switch', action)
  await vi.waitFor(() => expect(switcher.pending).not.toBeNull())

  switcher.confirm()
  await vi.waitFor(() => expect(stub.cancelCalls).toHaveBeenCalledTimes(1))
  // The cancel command round-tripped, but the run's own report has not
  // landed, so the action must not have run yet.
  expect(action).not.toHaveBeenCalled()
  expect(switcher.stopping).toBe(true)

  order.push('settle')
  stub.calls[0].settle(cancelled(0))
  await guarding

  expect(order).toEqual(['settle', 'action'])
  expect(imports.runs).toEqual([])
  expect(switcher.pending).toBeNull()
  expect(switcher.stopping).toBe(false)
})

it('a second guard while a question is up is dropped, and the first still answers', async () => {
  const stub = stubbedImports()
  const imports = new Imports()
  const switcher = new LibrarySwitch(imports)
  const first = vi.fn().mockResolvedValue(undefined)
  const second = vi.fn().mockResolvedValue(undefined)

  imports.enqueueBundle(['/a.db'])
  await stub.started(1)

  const firstCall = switcher.guard('switch', first)
  await vi.waitFor(() => expect(switcher.pending).not.toBeNull())
  await switcher.guard('switch', second)
  expect(second).not.toHaveBeenCalled()
  expect(switcher.pending).not.toBeNull()

  switcher.decline()
  await firstCall
  expect(first).not.toHaveBeenCalled()
  expect(imports.runs).toHaveLength(1)
})

it('a confirmed switch with work queued runs the action into an empty queue, starting none of it', async () => {
  const stub = stubbedImports()
  const imports = new Imports()
  const switcher = new LibrarySwitch(imports)
  // What the spec's "Confirming with work queued" scenario turns on: the swap
  // must not run while a waiting run is still there for `#pump` to start into
  // the library that is about to be opened.
  let queueAtSwap: number | null = null
  const action = vi.fn(async () => {
    queueAtSwap = imports.runs.length
  })

  imports.enqueueBundle(['/a.db'])
  imports.enqueue(['/second'])
  imports.enqueue(['/third'])
  await stub.started(1)

  const guarding = switcher.guard('switch', action)
  await vi.waitFor(() => expect(switcher.pending).not.toBeNull())
  expect(switcher.pending).toMatchObject({ kind: 'bundle', queued: 2 })

  switcher.confirm()
  await vi.waitFor(() => expect(stub.cancelCalls).toHaveBeenCalledTimes(1))
  stub.calls[0].settle(cancelled(0))
  await guarding

  expect(queueAtSwap).toBe(0)
  expect(stub.calls).toHaveLength(1)
  expect(imports.runs).toEqual([])
})
