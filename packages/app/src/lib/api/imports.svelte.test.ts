// @vitest-environment jsdom
// The queue, not the import itself: `import_paths` is stubbed, and what is
// asserted is that a second import asked for during a run waits its turn,
// runs, and leaves its own report.

import type { ImportReport } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { Imports } from './imports.svelte'
import { library } from './library.svelte'
import { status } from './status-fixture'

afterEach(() => {
  clearMocks()
})

function finished(imported: number): ImportReport {
  return { imported, skipped: 0, failed: 0, items: [] }
}

/**
 * Holds every `import_paths` call open until the test settles it, so the second
 * enqueue happens while the first run is genuinely still going.
 */
function stubbedImports() {
  const calls: {
    paths: string[]
    settle: (report: ImportReport) => void
    fail: (reason: string) => void
  }[] = []
  const statusCalls = vi.fn()

  mockIPC((cmd, args) => {
    if (cmd === 'import_paths') {
      const { paths } = args as { paths: string[] }
      return new Promise<ImportReport>((settle, fail) => {
        calls.push({ paths, settle, fail: (reason) => fail(reason) })
      })
    }
    if (cmd === 'library_status') {
      statusCalls()
      return status({ imageCount: statusCalls.mock.calls.length })
    }
    throw `unexpected command ${cmd}`
  }, { shouldMockEvents: true })

  return {
    calls,
    statusCalls,
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
  expect(stub.calls[0].paths).toEqual(['/first'])

  stub.calls[0].settle(finished(1))
  await stub.started(2)

  expect(stub.calls[1].paths).toEqual(['/second'])
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
