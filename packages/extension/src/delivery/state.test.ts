import { expect, it } from 'vitest'

import {
  beginDelivery,
  createEntry,
  markDelivered,
  markFailed,
  SWEEP_REASON,
  sweepPending,
  type CaptureEntry,
} from './state.js'

function entry(overrides: Partial<CaptureEntry> = {}): CaptureEntry {
  return {
    ...createEntry({
      id: 'id-1',
      imageUrl: 'https://cdn.test/a.png',
      pageUrl: 'https://page.test/gallery',
      pageTitle: 'a page',
      capturedAt: 1_700_000_000_000,
      size: 3,
      thumbnail: 'data:image/jpeg;base64,AAAA',
    }),
    ...overrides,
  }
}

it('starts a capture pending', () => {
  expect(entry().status).toBe('pending')
})

it('takes a pending capture to delivered', () => {
  expect(markDelivered(entry()).status).toBe('delivered')
})

it('takes a pending capture to failed, with the reason', () => {
  const failed = markFailed(entry(), 'Could not reach the app')

  expect(failed.status).toBe('failed')
  expect(failed.reason).toBe('Could not reach the app')
})

it('takes a failed capture back into flight on a retry', () => {
  const retried = beginDelivery(markFailed(entry(), 'Could not reach the app'))

  expect(retried.status).toBe('pending')
  expect(retried.reason).toBeUndefined()
})

it('clears the reason once a retry succeeds', () => {
  const delivered = markDelivered(markFailed(entry(), 'Could not reach the app'))

  expect(delivered.status).toBe('delivered')
  expect(delivered.reason).toBeUndefined()
})

it('leaves a delivered capture delivered', () => {
  const delivered = markDelivered(entry())

  expect(markFailed(delivered, 'too late').status).toBe('delivered')
  expect(beginDelivery(delivered).status).toBe('delivered')
  expect(markDelivered(delivered)).toEqual(delivered)
})

it('fails every capture still pending when the worker starts', () => {
  const entries = [
    entry({ id: 'a', status: 'pending' }),
    entry({ id: 'b', status: 'delivered' }),
    entry({ id: 'c', status: 'failed', reason: 'earlier' }),
    entry({ id: 'd', status: 'pending' }),
  ]

  const swept = sweepPending(entries)

  expect(swept.map((e) => e.id)).toEqual(['a', 'd'])
  expect(swept.every((e) => e.status === 'failed')).toBe(true)
  expect(swept[0]!.reason).toBe(SWEEP_REASON)
})

it('does not touch entries that were never in flight', () => {
  expect(sweepPending([entry({ status: 'delivered' }), entry({ status: 'failed' })])).toEqual([])
})
