// @vitest-environment jsdom

import type { CaptureMeta, ImageRecord } from '@boorubox/shared'
import { emit } from '@tauri-apps/api/event'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it } from 'vitest'
import {
  CAPTURE_PENDING_EVENT,
  CAPTURE_STORED_EVENT,
  CAPTURE_WITHDRAWN_EVENT,
} from './events'
import { PENDING_TTL_MS, PendingCaptures, type Clock } from './pending.svelte'

afterEach(() => {
  clearMocks()
})

function meta(id: string): CaptureMeta {
  return {
    id,
    imageUrl: `https://pbs.example/media/${id}.jpg`,
    pageUrl: `https://x.com/an/status/${id}`,
    pageTitle: `post ${id}`,
    capturedAt: 1_700_000_000_000,
  }
}

/** The `Clock` the store schedules its expiries on, under the test's hand. */
function stoppedClock() {
  const scheduled: { delay: number, run: () => void, live: boolean }[] = []
  const clock: Clock = (run, delay) => {
    const task = { delay, run, live: true }
    scheduled.push(task)
    return () => (task.live = false)
  }
  return {
    clock,
    live: () => scheduled.filter((task) => task.live),
    /** Fires everything still scheduled, as the real timers would at the bound. */
    fire: () => scheduled.filter((task) => task.live).forEach((task) => {
      task.live = false
      task.run()
    }),
  }
}

async function subscribed(clock: Clock = stoppedClock().clock) {
  mockIPC(() => {}, { shouldMockEvents: true })
  const pending = new PendingCaptures(clock)
  return { pending, unlisten: await pending.subscribe() }
}

it('shows an announced capture at once, newest first', async () => {
  const { pending } = await subscribed()

  await emit(CAPTURE_PENDING_EVENT, meta('cap-1'))
  await emit(CAPTURE_PENDING_EVENT, meta('cap-2'))

  expect(pending.entries.map((entry) => entry.id)).toEqual(['cap-2', 'cap-1'])
  expect(pending.entries[0].pageTitle).toBe('post cap-2')
})

it('clears only the id that was stored', async () => {
  const { pending } = await subscribed()

  await emit(CAPTURE_PENDING_EVENT, meta('cap-1'))
  await emit(CAPTURE_PENDING_EVENT, meta('cap-2'))
  await emit(CAPTURE_STORED_EVENT, { id: 'cap-1' } as ImageRecord)

  expect(pending.entries.map((entry) => entry.id)).toEqual(['cap-2'])
})

it('clears only the id that was withdrawn', async () => {
  const { pending } = await subscribed()

  await emit(CAPTURE_PENDING_EVENT, meta('cap-1'))
  await emit(CAPTURE_PENDING_EVENT, meta('cap-2'))
  await emit(CAPTURE_WITHDRAWN_EVENT, { id: 'cap-2', reason: 'the fetch failed' })

  expect(pending.entries.map((entry) => entry.id)).toEqual(['cap-1'])
})

it('holds one entry for an id announced twice', async () => {
  const { pending } = await subscribed()

  await emit(CAPTURE_PENDING_EVENT, meta('cap-1'))
  await emit(CAPTURE_PENDING_EVENT, { ...meta('cap-1'), pageTitle: 'the same post, renamed' })

  expect(pending.entries).toHaveLength(1)
  expect(pending.entries[0].pageTitle).toBe('the same post, renamed')
})

it('drops an announcement nothing settled, at the bound', async () => {
  const clock = stoppedClock()
  const { pending } = await subscribed(clock.clock)

  await emit(CAPTURE_PENDING_EVENT, meta('cap-1'))
  expect(clock.live().map((task) => task.delay)).toEqual([PENDING_TTL_MS])

  clock.fire()
  expect(pending.entries).toEqual([])
})

it('does not expire an announcement that was already settled', async () => {
  const clock = stoppedClock()
  const { pending } = await subscribed(clock.clock)

  await emit(CAPTURE_PENDING_EVENT, meta('cap-1'))
  await emit(CAPTURE_STORED_EVENT, { id: 'cap-1' } as ImageRecord)
  await emit(CAPTURE_PENDING_EVENT, meta('cap-2'))

  expect(clock.live()).toHaveLength(1)
  clock.fire()
  expect(pending.entries).toEqual([])
})

it('cancels the timers it is still holding when it stops listening', async () => {
  const clock = stoppedClock()
  const { pending, unlisten } = await subscribed(clock.clock)

  await emit(CAPTURE_PENDING_EVENT, meta('cap-1'))
  await unlisten()

  expect(clock.live()).toEqual([])
  expect(pending.entries).toEqual([])
})
