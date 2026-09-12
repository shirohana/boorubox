// @vitest-environment jsdom
// `check()`/`download()`/`install()`/`relaunch()` all go through the same
// `invoke` the rest of the app's commands do (`plugin:updater|check`,
// `plugin:updater|download`, `plugin:updater|install`,
// `plugin:process|restart`, and `plugin:resources|close` for a closed
// handle), so they are stubbed with `mockIPC` exactly like a Rust command —
// no need to mock the plugin modules themselves.

import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { AppUpdate } from './update.svelte'

afterEach(() => {
  clearMocks()
})

const METADATA = {
  rid: 1,
  currentVersion: '26.9.1201',
  version: '26.9.1202',
  date: undefined,
  body: 'A same-day fix.',
  rawJson: {},
}

/** Nothing is ever in flight, unless a test hands `install` something else. */
const NEVER_BUSY = () => null

/**
 * `check: undefined` answers "nothing newer" (the real command's own answer
 * for that case — `null`, not the absence of a mock); `check: 'reject'`
 * throws, standing in for "no network" or an unreachable manifest.
 */
function stubbedUpdater(options: {
  check?: typeof METADATA | 'reject'
  download?: () => unknown
  install?: () => unknown
} = {}) {
  const checkCalls = vi.fn()
  const downloadCalls = vi.fn()
  const installCalls = vi.fn()
  const relaunchCalls = vi.fn()
  const closeCalls = vi.fn()
  let nextRid = 100
  mockIPC((cmd) => {
    if (cmd === 'plugin:updater|check') {
      checkCalls()
      if (options.check === 'reject') throw new Error('no network')
      return options.check ?? null
    }
    if (cmd === 'plugin:updater|download') {
      downloadCalls()
      return options.download ? options.download() : nextRid++
    }
    if (cmd === 'plugin:updater|install') {
      installCalls()
      return options.install?.()
    }
    if (cmd === 'plugin:process|restart') {
      relaunchCalls()
      return undefined
    }
    if (cmd === 'plugin:resources|close') {
      closeCalls()
      return undefined
    }
    throw new Error(`unexpected command ${cmd}`)
  })
  return { checkCalls, downloadCalls, installCalls, relaunchCalls, closeCalls }
}

it('a launch check that finds nothing leaves the prompt closed', async () => {
  stubbedUpdater()
  const appUpdate = new AppUpdate()

  await appUpdate.checkOnLaunch()

  expect(appUpdate.available).toBeNull()
  expect(appUpdate.promptOpen).toBe(false)
})

it('a launch check that finds an update opens the prompt', async () => {
  stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()

  await appUpdate.checkOnLaunch()

  expect(appUpdate.available?.version).toBe('26.9.1202')
  expect(appUpdate.promptOpen).toBe(true)
})

it('a launch check runs once even if asked twice', async () => {
  const stub = stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()

  await appUpdate.checkOnLaunch()
  await appUpdate.checkOnLaunch()

  expect(stub.checkCalls).toHaveBeenCalledTimes(1)
})

it('a launch check that fails is silent: no error surfaces anywhere', async () => {
  stubbedUpdater({ check: 'reject' })
  const appUpdate = new AppUpdate()

  await expect(appUpdate.checkOnLaunch()).resolves.toBeUndefined()

  expect(appUpdate.available).toBeNull()
  expect(appUpdate.lastCheckError).toBeNull()
  expect(appUpdate.promptOpen).toBe(false)
})

it('an explicit check reports "current" when nothing is newer', async () => {
  stubbedUpdater()
  const appUpdate = new AppUpdate()

  const outcome = await appUpdate.checkNow()

  expect(outcome).toBe('current')
  expect(appUpdate.checking).toBe(false)
  expect(appUpdate.available).toBeNull()
})

it('an explicit check reports "failed" and keeps the reason', async () => {
  stubbedUpdater({ check: 'reject' })
  const appUpdate = new AppUpdate()

  const outcome = await appUpdate.checkNow()

  expect(outcome).toBe('failed')
  expect(appUpdate.lastCheckError).toBe('no network')
})

it('an explicit check reports "available" and opens the prompt', async () => {
  stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()

  const outcome = await appUpdate.checkNow()

  expect(outcome).toBe('available')
  expect(appUpdate.available?.version).toBe('26.9.1202')
  expect(appUpdate.promptOpen).toBe(true)
})

it('a second explicit check while one is running shares the first one\'s answer, not a second request', async () => {
  const stub = stubbedUpdater()
  const appUpdate = new AppUpdate()

  const first = appUpdate.checkNow()
  const second = appUpdate.checkNow()

  expect(await first).toBe('current')
  expect(await second).toBe('current')
  expect(stub.checkCalls).toHaveBeenCalledTimes(1)
})

it('declining closes the prompt, drops the update, and closes its handle', async () => {
  const stub = stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()

  appUpdate.decline()

  expect(appUpdate.promptOpen).toBe(false)
  expect(appUpdate.declined).toBe(true)
  expect(appUpdate.available).toBeNull()
  // `close()` is fire-and-forget from `decline()` (nothing here awaits it),
  // so it lands a tick later.
  await vi.waitFor(() => expect(stub.closeCalls).toHaveBeenCalledTimes(1))
})

it('reopen with nothing on offer does nothing', () => {
  stubbedUpdater()
  const appUpdate = new AppUpdate()

  appUpdate.reopen()

  expect(appUpdate.promptOpen).toBe(false)
})

it('reopen brings the prompt back for an update found again after an earlier decline', async () => {
  stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()
  appUpdate.decline()
  expect(appUpdate.available).toBeNull()

  // Declining does not turn off future discovery, only the automatic prompt
  // (design D5) — an explicit check still surfaces the update, just without
  // reopening the dialog on its own.
  const outcome = await appUpdate.checkNow()
  expect(outcome).toBe('available')
  expect(appUpdate.promptOpen).toBe(false)

  appUpdate.reopen()

  expect(appUpdate.promptOpen).toBe(true)
})

it('installing while work is in flight downloads nothing and reports the refusal', async () => {
  const stub = stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()

  await appUpdate.install(() => 'an import is still running')

  expect(stub.downloadCalls).not.toHaveBeenCalled()
  expect(stub.installCalls).not.toHaveBeenCalled()
  expect(stub.relaunchCalls).not.toHaveBeenCalled()
  expect(appUpdate.installing).toBe(false)
  expect(appUpdate.installError).toContain('an import is still running')
  // The refusal before any download starts touches nothing on offer.
  expect(appUpdate.available).not.toBeNull()
})

it('installing with nothing in flight downloads, installs and restarts', async () => {
  const stub = stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()

  await appUpdate.install(NEVER_BUSY)

  expect(stub.downloadCalls).toHaveBeenCalledTimes(1)
  expect(stub.installCalls).toHaveBeenCalledTimes(1)
  expect(stub.relaunchCalls).toHaveBeenCalledTimes(1)
  expect(appUpdate.installError).toBeNull()
  // The real app has restarted by now; nothing resets `installing` back —
  // there is nothing left running that would.
  expect(appUpdate.installing).toBe(true)
})

it('work starting during the download refuses right before install, and keeps the download', async () => {
  const stub = stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()

  // Free when `install()` makes its first check (before downloading, the
  // cheap refusal); busy by the time it checks again immediately before
  // `Update#install()` — exactly the window the download's own minutes open
  // up (design D6: a real refusal, not a delay-and-hope).
  let asked = 0
  const busyWith = () => (asked++ === 0 ? null : 'an import is still running')

  await appUpdate.install(busyWith)

  expect(stub.downloadCalls).toHaveBeenCalledTimes(1)
  expect(stub.installCalls).not.toHaveBeenCalled()
  expect(stub.relaunchCalls).not.toHaveBeenCalled()
  expect(appUpdate.installing).toBe(false)
  expect(appUpdate.installError).toContain('an import is still running')
  // Not discarded: the bytes are already down, ready for the next attempt.
  expect(appUpdate.available).not.toBeNull()
  expect(appUpdate.downloaded).toBe(true)
})

it('a later install, once free, finishes from what was already downloaded rather than fetching it again', async () => {
  const stub = stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()

  let asked = 0
  await appUpdate.install(() => (asked++ === 0 ? null : 'an import is still running'))
  expect(appUpdate.downloaded).toBe(true)

  await appUpdate.install(NEVER_BUSY)

  expect(stub.downloadCalls).toHaveBeenCalledTimes(1)
  expect(stub.installCalls).toHaveBeenCalledTimes(1)
  expect(stub.relaunchCalls).toHaveBeenCalledTimes(1)
})

it('install is a no-op while one is already running — no second installer over the same app', async () => {
  const stub = stubbedUpdater({ check: METADATA })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()

  const first = appUpdate.install(NEVER_BUSY)
  // Synchronous: `installing` is already true by the time this runs, before
  // `first` has had a chance to resolve.
  await appUpdate.install(NEVER_BUSY)
  await first

  expect(stub.downloadCalls).toHaveBeenCalledTimes(1)
  expect(stub.installCalls).toHaveBeenCalledTimes(1)
})

it('a download failure is reported and drops the update, without installing', async () => {
  const stub = stubbedUpdater({
    check: METADATA,
    download: () => {
      throw new Error('no network')
    },
  })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()

  await appUpdate.install(NEVER_BUSY)

  expect(appUpdate.installError).toBe('no network')
  expect(appUpdate.installing).toBe(false)
  expect(appUpdate.available).toBeNull()
  expect(stub.installCalls).not.toHaveBeenCalled()
  expect(stub.relaunchCalls).not.toHaveBeenCalled()
})

it('a failed install (signature verification) is reported and drops the update, without restarting', async () => {
  const stub = stubbedUpdater({
    check: METADATA,
    install: () => {
      throw new Error('signature verification failed')
    },
  })
  const appUpdate = new AppUpdate()
  await appUpdate.checkOnLaunch()

  await appUpdate.install(NEVER_BUSY)

  expect(appUpdate.installError).toBe('signature verification failed')
  expect(appUpdate.installing).toBe(false)
  expect(appUpdate.available).toBeNull()
  expect(stub.relaunchCalls).not.toHaveBeenCalled()
})

it('installing with nothing on offer does nothing', async () => {
  const stub = stubbedUpdater()
  const appUpdate = new AppUpdate()

  await appUpdate.install(NEVER_BUSY)

  expect(stub.downloadCalls).not.toHaveBeenCalled()
  expect(appUpdate.installing).toBe(false)
})
