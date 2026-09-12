import { expect, it } from 'vitest'
import { workInFlight } from './update-work'

it('names nothing when nothing is running', () => {
  expect(workInFlight(0, 0)).toBeNull()
})

it('names an import alone', () => {
  expect(workInFlight(1, 0)).toBe('an import is still running')
})

it('names a capture alone', () => {
  expect(workInFlight(0, 1)).toBe('a capture is still running')
})

it('names both, plural verb, when an import and a capture are both running', () => {
  expect(workInFlight(3, 2)).toBe('an import and a capture are still running')
})
