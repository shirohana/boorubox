// @vitest-environment jsdom

import { emit } from '@tauri-apps/api/event'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it } from 'vitest'
import { SIDECARS_PROGRESS_EVENT } from './events'
import { SidecarsBackfill } from './sidecars-backfill.svelte'

afterEach(() => {
  clearMocks()
})

async function subscribed() {
  mockIPC(() => {}, { shouldMockEvents: true })
  const backfill = new SidecarsBackfill()
  return { backfill, unlisten: await backfill.subscribe() }
}

it('shows no tile for a library already in step, which emits nothing', async () => {
  const { backfill } = await subscribed()
  expect(backfill.progress).toBeNull()
})

it('tracks a running pass', async () => {
  const { backfill } = await subscribed()

  await emit(SIDECARS_PROGRESS_EVENT, { done: 100, total: 25_000 })

  expect(backfill.progress).toEqual({ done: 100, total: 25_000 })
})

it('goes at once on the last tick, leaving no result to dismiss', async () => {
  const { backfill } = await subscribed()

  await emit(SIDECARS_PROGRESS_EVENT, { done: 100, total: 400 })
  expect(backfill.progress).not.toBeNull()

  await emit(SIDECARS_PROGRESS_EVENT, { done: 400, total: 400 })

  expect(backfill.progress).toBeNull()
})

it('reset clears a running pass, for a library switch or close', async () => {
  const { backfill } = await subscribed()

  await emit(SIDECARS_PROGRESS_EVENT, { done: 5, total: 400 })
  expect(backfill.progress).not.toBeNull()

  backfill.reset()

  expect(backfill.progress).toBeNull()
})

it('stops delivering once unlistened', async () => {
  const { backfill, unlisten } = await subscribed()
  await unlisten()

  await emit(SIDECARS_PROGRESS_EVENT, { done: 5, total: 400 })

  expect(backfill.progress).toBeNull()
})
