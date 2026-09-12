// @vitest-environment jsdom
// The queue, not the import itself: `import_paths`/`import_bundle` are
// stubbed, and what is asserted is queue behaviour — ordering, progress and
// reports — the same for both run kinds (`legacy-bundle-import` design D6).

import type { ImportReport } from '@boorubox/shared'
import { emit } from '@tauri-apps/api/event'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { IMPORT_PROGRESS_EVENT } from './events'
import { Imports } from './imports.svelte'
import { library } from './library.svelte'
import { status } from './status-fixture'

afterEach(() => {
  clearMocks()
})

function finished(imported: number): ImportReport {
  return { imported, skipped: 0, failed: 0, cancelled: false, items: [] }
}

function cancelled(imported: number): ImportReport {
  return { imported, skipped: 0, failed: 0, cancelled: true, items: [] }
}

/**
 * Holds every `import_paths`/`import_bundle` call open until the test settles
 * it, so a later enqueue happens while the first run is genuinely still going.
 */
function stubbedImports() {
  const calls: {
    kind: 'paths' | 'bundle'
    items: string[]
    settle: (report: ImportReport) => void
    fail: (reason: string) => void
  }[] = []
  const statusCalls = vi.fn()
  const pauseCalls = vi.fn()
  const resumeCalls = vi.fn()
  const cancelCalls = vi.fn()

  mockIPC((cmd, args) => {
    if (cmd === 'import_paths' || cmd === 'import_bundle') {
      const items = cmd === 'import_paths'
        ? (args as { paths: string[] }).paths
        : (args as { files: string[] }).files
      return new Promise<ImportReport>((settle, fail) => {
        calls.push({
          kind: cmd === 'import_paths' ? 'paths' : 'bundle',
          items,
          settle,
          fail: (reason) => fail(reason),
        })
      })
    }
    if (cmd === 'library_status') {
      statusCalls()
      return status({ imageCount: statusCalls.mock.calls.length })
    }
    if (cmd === 'import_pause') return pauseCalls()
    if (cmd === 'import_resume') return resumeCalls()
    if (cmd === 'import_cancel') return cancelCalls()
    throw `unexpected command ${cmd}`
  }, { shouldMockEvents: true })

  return {
    calls,
    statusCalls,
    pauseCalls,
    resumeCalls,
    cancelCalls,
    /** Resolves once `n` runs have reached the stub. */
    started: (n: number) => vi.waitFor(() => expect(calls).toHaveLength(n)),
  }
}

it('runs a second import asked for during the first, in order', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])

  expect(imports.runs.map((run) => run.status)).toEqual(['running', 'queued'])
  await stub.started(1)
  expect(stub.calls[0].items).toEqual(['/first'])

  stub.calls[0].settle(finished(1))
  await stub.started(2)

  expect(stub.calls[1].items).toEqual(['/second'])
  expect(imports.runs.map((run) => run.status)).toEqual(['running'])

  stub.calls[1].settle(finished(2))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
})

it('keeps a report per run, newest first', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])
  await stub.started(1)
  stub.calls[0].settle(finished(1))
  await stub.started(2)
  stub.calls[1].settle(finished(2))

  await vi.waitFor(() => expect(imports.reports).toHaveLength(2))
  expect(imports.reports.map((report) => report.imported)).toEqual([2, 1])

  imports.dismiss(imports.reports[1])
  expect(imports.reports.map((report) => report.imported)).toEqual([2])
})

it('tells its listener once per run, and stops when it unsubscribes', async () => {
  const stub = stubbedImports()
  const imports = new Imports()
  const refresh = vi.fn()
  const unsubscribe = imports.onfinished(refresh)

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])
  await stub.started(1)
  stub.calls[0].settle(finished(1))
  await stub.started(2)
  stub.calls[1].settle(finished(2))
  await vi.waitFor(() => expect(refresh).toHaveBeenCalledTimes(2))

  unsubscribe()
  imports.enqueue(['/third'])
  await stub.started(3)
  stub.calls[2].settle(finished(3))
  await vi.waitFor(() => expect(imports.reports).toHaveLength(3))
  expect(refresh).toHaveBeenCalledTimes(2)
})

it('reads the library status back after every run', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])
  await stub.started(1)
  stub.calls[0].settle(finished(1))
  await vi.waitFor(() => expect(library.status?.imageCount).toBe(1))

  await stub.started(2)
  stub.calls[1].settle(finished(2))
  await vi.waitFor(() => expect(library.status?.imageCount).toBe(2))
  expect(stub.statusCalls).toHaveBeenCalledTimes(2)
})

it('queues nothing for no paths', () => {
  stubbedImports()
  const imports = new Imports()

  imports.enqueue([])

  expect(imports.runs).toEqual([])
  expect(imports.error).toBeNull()
})

it('a cancelled picker queues nothing and is not an error', async () => {
  stubbedImports()
  const imports = new Imports()

  await imports.pick(async () => [])

  expect(imports.runs).toEqual([])
  expect(imports.error).toBeNull()
})

it('reports a failed run and still starts the one behind it', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])
  await stub.started(1)
  stub.calls[0].fail('no library is open')
  await stub.started(2)

  expect(imports.error).toBe('no library is open')

  stub.calls[1].settle(finished(2))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
  expect(imports.reports.map((report) => report.imported)).toEqual([2])
})

it('runs a bundle import, landing its report as the latest bundle report', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueueBundle(['/bundle/database-part1of2.db'])

  expect(imports.runs.map((run) => run.kind)).toEqual(['bundle'])
  await stub.started(1)
  expect(stub.calls[0]).toMatchObject({
    kind: 'bundle',
    items: ['/bundle/database-part1of2.db'],
  })

  stub.calls[0].settle(finished(3))
  await vi.waitFor(() => expect(imports.reports).toHaveLength(1))
  expect(imports.latestBundleReport?.imported).toBe(3)
  expect(imports.reports[0].imported).toBe(3)
})

it('tracks a bundle run\'s progress on its own queue entry', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueueBundle(['/bundle/database.db'])
  await stub.started(1)

  const progress = { done: 2, total: 4, imported: 1, skipped: 1, failed: 0 }
  await emit(IMPORT_PROGRESS_EVENT, progress)
  await vi.waitFor(() => expect(imports.runs[0]?.progress?.done).toBe(2))
  expect(imports.runs[0]?.progress).toEqual(progress)

  stub.calls[0].settle(finished(1))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
})

it('a cancelled bundle picker queues nothing and is not an error', async () => {
  stubbedImports()
  const imports = new Imports()

  await imports.pickBundle(async () => [])

  expect(imports.runs).toEqual([])
  expect(imports.error).toBeNull()
})

it('runs a bundle import asked for during a paths import, in order', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/local'])
  imports.enqueueBundle(['/bundle/database.db'])

  expect(imports.runs.map((run) => run.kind)).toEqual(['paths', 'bundle'])
  await stub.started(1)
  expect(stub.calls[0].kind).toBe('paths')

  stub.calls[0].settle(finished(1))
  await stub.started(2)

  expect(stub.calls[1].kind).toBe('bundle')
  expect(imports.runs.map((run) => run.kind)).toEqual(['bundle'])

  stub.calls[1].settle(finished(2))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
  expect(imports.reports.map((report) => report.imported)).toEqual([2, 1])
  expect(imports.latestBundleReport?.imported).toBe(2)
})

it('a report carries the run\'s own kind, for the two different re-run wordings', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueueBundle(['/bundle/database.db'])
  await stub.started(1)
  stub.calls[0].settle(finished(1))

  await vi.waitFor(() => expect(imports.reports).toHaveLength(1))
  expect(imports.reports[0].kind).toBe('bundle')
})

it('pausing does not start a queued run, and resuming does not double-start it', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])
  await stub.started(1)

  await imports.pause()
  expect(stub.pauseCalls).toHaveBeenCalledTimes(1)
  expect(imports.paused).toBe(true)
  // Still only the first run has reached the stub: pausing never starts what
  // is queued behind it.
  expect(stub.calls).toHaveLength(1)

  await imports.resume()
  expect(stub.resumeCalls).toHaveBeenCalledTimes(1)
  expect(imports.paused).toBe(false)
  // Resume only unparks the run already going; it does not itself start the
  // next one, which only starts once the first settles.
  expect(stub.calls).toHaveLength(1)

  stub.calls[0].settle(finished(1))
  await stub.started(2)
  expect(stub.calls[1].items).toEqual(['/second'])

  stub.calls[1].settle(finished(2))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
})

it('cancelling drops every queued run and counts them for the report it shows', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])
  imports.enqueue(['/third'])
  await stub.started(1)

  await imports.cancel()
  expect(stub.cancelCalls).toHaveBeenCalledTimes(1)
  // Neither queued run is queued nor running: both are simply gone.
  expect(imports.runs.map((run) => run.status)).toEqual(['running'])

  stub.calls[0].settle(cancelled(1))
  await vi.waitFor(() => expect(imports.reports).toHaveLength(1))
  expect(imports.reports[0].cancelled).toBe(true)
  expect(imports.reports[0].queuedDiscarded).toBe(2)
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
})

it('a cancel with nothing queued reports zero discarded', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  await stub.started(1)

  await imports.cancel()
  stub.calls[0].settle(cancelled(1))

  await vi.waitFor(() => expect(imports.reports).toHaveLength(1))
  expect(imports.reports[0].queuedDiscarded).toBe(0)
})

it('does not leak a discarded-queue count into the next, unrelated run when the cancelled command rejects', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])
  await stub.started(1)

  await imports.cancel()
  stub.calls[0].fail('write conflict')
  // `/second` was already dropped by the cancel, so nothing runs until a
  // fresh, unrelated import is asked for.
  await vi.waitFor(() => expect(imports.runs).toEqual([]))

  imports.enqueue(['/third'])
  await stub.started(2)
  stub.calls[1].settle(finished(1))

  await vi.waitFor(() => expect(imports.reports).toHaveLength(1))
  expect(imports.reports[0].queuedDiscarded).toBe(0)
})

it('replays a cancel pressed before the run\'s first progress event', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  await stub.started(1)

  await imports.cancel()
  expect(stub.cancelCalls).toHaveBeenCalledTimes(1)

  await emit(IMPORT_PROGRESS_EVENT, { done: 1, total: 3, imported: 1, skipped: 0, failed: 0 })
  await vi.waitFor(() => expect(stub.cancelCalls).toHaveBeenCalledTimes(2))

  // A later tick does not replay again — the one outstanding request was
  // latched and cleared by the first.
  await emit(IMPORT_PROGRESS_EVENT, { done: 2, total: 3, imported: 2, skipped: 0, failed: 0 })
  await vi.waitFor(() => expect(imports.runs[0]?.progress?.done).toBe(2))
  expect(stub.cancelCalls).toHaveBeenCalledTimes(2)

  stub.calls[0].settle(cancelled(2))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
})

it('replays a pause pressed before the run\'s first progress event', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  await stub.started(1)

  await imports.pause()
  expect(stub.pauseCalls).toHaveBeenCalledTimes(1)

  await emit(IMPORT_PROGRESS_EVENT, { done: 1, total: 3, imported: 1, skipped: 0, failed: 0 })
  await vi.waitFor(() => expect(stub.pauseCalls).toHaveBeenCalledTimes(2))

  stub.calls[0].settle(finished(1))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
})

it('a resolved cancel does not replay a stale request into the next run', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  await stub.started(1)

  await imports.pause()
  // The pause never got its confirming progress tick, so it is still
  // latched when the run ends some other way (settled without ever ticking).
  stub.calls[0].settle(finished(1))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))

  imports.enqueue(['/second'])
  await stub.started(2)
  await emit(IMPORT_PROGRESS_EVENT, { done: 1, total: 2, imported: 1, skipped: 0, failed: 0 })
  await vi.waitFor(() => expect(imports.runs[0]?.progress?.done).toBe(1))
  // The stale pause from the first run must not have carried over.
  expect(stub.pauseCalls).toHaveBeenCalledTimes(1)

  stub.calls[1].settle(finished(1))
  await vi.waitFor(() => expect(imports.runs).toEqual([]))
})

it('dequeue removes only the named waiting run, leaving everything else untouched', async () => {
  const stub = stubbedImports()
  const imports = new Imports()

  imports.enqueue(['/first'])
  imports.enqueue(['/second'])
  imports.enqueue(['/third'])
  await stub.started(1)

  const secondId = imports.runs[1]?.id
  if (secondId === undefined) throw new Error('expected a second, queued run')
  imports.dequeue(secondId)

  expect(imports.runs.map((run) => run.status)).toEqual(['running', 'queued'])
  expect(stub.cancelCalls).not.toHaveBeenCalled()

  stub.calls[0].settle(finished(1))
  await stub.started(2)
  expect(stub.calls[1].items).toEqual(['/third'])

  stub.calls[1].settle(finished(2))
  await vi.waitFor(() => expect(imports.reports).toHaveLength(2))
  // Dequeuing a waiting run is not a discard the running run's report should
  // ever hear about.
  expect(imports.reports.every((report) => report.queuedDiscarded === 0)).toBe(true)
})
