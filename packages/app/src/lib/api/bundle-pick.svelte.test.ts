// @vitest-environment jsdom
// `imports.enqueueBundle` is spied rather than run for real: what these tests
// assert is the pick/plan/confirm/discard behaviour, not the run queue, which
// `imports.svelte.test.ts` already covers on its own.

import type { BundlePlan } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { BundlePick } from './bundle-pick.svelte'
import { imports } from './imports.svelte'

afterEach(() => {
  clearMocks()
  vi.restoreAllMocks()
})

function plan(parts: BundlePlan['parts']): BundlePlan {
  return { parts, total: parts.reduce((sum, part) => sum + (part.rows ?? 1), 0) }
}

it('picking plans the files and does not enqueue', async () => {
  const calls = vi.fn()
  mockIPC((cmd, args) => {
    calls(cmd, args)
    return plan([{ path: '/b.db', rows: 3, error: null }])
  })
  const enqueueBundle = vi.spyOn(imports, 'enqueueBundle').mockImplementation(() => {})
  const pick = new BundlePick()

  await pick.pick(async () => ['/b.db'])

  expect(calls).toHaveBeenCalledWith('bundle_plan', { files: ['/b.db'] })
  expect(pick.plan?.parts).toEqual([{ path: '/b.db', rows: 3, error: null }])
  expect(pick.planning).toBe(false)
  expect(pick.error).toBeNull()
  expect(enqueueBundle).not.toHaveBeenCalled()
})

it('a cancelled picker plans nothing and is not an error', async () => {
  mockIPC(() => {
    throw 'should not be called'
  })
  const pick = new BundlePick()

  await pick.pick(async () => [])

  expect(pick.plan).toBeNull()
  expect(pick.error).toBeNull()
})

it('confirm enqueues exactly the planned parts, in their order, and clears the pick', async () => {
  mockIPC((cmd) => {
    if (cmd === 'bundle_plan') {
      return plan([
        { path: '/part1.db', rows: 2, error: null },
        { path: '/part2.db', rows: 4, error: null },
      ])
    }
    throw `unexpected command ${cmd}`
  })
  const enqueueBundle = vi.spyOn(imports, 'enqueueBundle').mockImplementation(() => {})
  const pick = new BundlePick()

  // The picker's own order need not survive; `bundle_plan`'s is the one the
  // run honours.
  await pick.pick(async () => ['/part2.db', '/part1.db'])
  pick.confirm()

  expect(enqueueBundle).toHaveBeenCalledExactlyOnceWith(['/part1.db', '/part2.db'])
  expect(pick.plan).toBeNull()
  expect(pick.files).toEqual([])
})

it('discard enqueues nothing and leaves no pick', async () => {
  mockIPC((cmd) => {
    if (cmd === 'bundle_plan') return plan([{ path: '/a.db', rows: 1, error: null }])
    throw `unexpected command ${cmd}`
  })
  const enqueueBundle = vi.spyOn(imports, 'enqueueBundle').mockImplementation(() => {})
  const pick = new BundlePick()

  await pick.pick(async () => ['/a.db'])
  pick.discard()

  expect(pick.plan).toBeNull()
  expect(pick.files).toEqual([])
  expect(enqueueBundle).not.toHaveBeenCalled()
})

it('confirm with no plan is a no-op', () => {
  const enqueueBundle = vi.spyOn(imports, 'enqueueBundle').mockImplementation(() => {})
  const pick = new BundlePick()

  expect(() => pick.confirm()).not.toThrow()

  expect(enqueueBundle).not.toHaveBeenCalled()
  expect(pick.plan).toBeNull()
})

it('a failed bundle_plan leaves the error and no pick to confirm', async () => {
  mockIPC(() => {
    throw 'not a valid legacy bundle'
  })
  const pick = new BundlePick()

  await pick.pick(async () => ['/broken.db'])

  expect(pick.error).toBe('not a valid legacy bundle')
  expect(pick.plan).toBeNull()
  expect(pick.planning).toBe(false)
})
