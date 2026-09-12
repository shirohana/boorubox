import { expect, it } from 'vitest'
import { queuedDiscardedNotice, rerunNotice } from './cancelled-import'

it('says a bundle re-run resumes (legacy-bundle-import "The user is told it resumes")', () => {
  expect(rerunNotice('bundle')).toBe(
    'Selecting the same parts again will carry on from where it stopped.',
  )
})

it('says a local re-run imports everything again, and offers no Resume in the wording', () => {
  expect(rerunNotice('paths')).toBe('Importing the same files again will import them again.')
  expect(rerunNotice('paths').toLowerCase()).not.toContain('resume')
})

it('names nothing when no queued run was discarded', () => {
  expect(queuedDiscardedNotice(0)).toBeNull()
})

it('says one queued import was discarded, singular', () => {
  expect(queuedDiscardedNotice(1)).toBe('1 queued import was discarded.')
})

it('says how many queued imports were discarded, plural', () => {
  expect(queuedDiscardedNotice(2)).toBe('2 queued imports were discarded.')
})
