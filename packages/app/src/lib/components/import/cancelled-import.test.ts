import { expect, it } from 'vitest'
import { queuedDiscardedNotice, rerunNotice, switchConfirmation } from './cancelled-import'

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

it('says a bundle switch confirmation carries the bundle re-run sentence', () => {
  const confirmation = switchConfirmation('bundle', 0, 'switch')
  expect(confirmation.description).toContain(rerunNotice('bundle'))
})

it('says a local switch confirmation carries the local re-run sentence, with no Resume implied', () => {
  const confirmation = switchConfirmation('paths', 0, 'switch')
  expect(confirmation.description).toContain(rerunNotice('paths'))
  expect(confirmation.description.toLowerCase()).not.toContain('resume')
})

it('names no queued clause with nothing waiting', () => {
  const confirmation = switchConfirmation('paths', 0, 'switch')
  expect(confirmation.description).not.toMatch(/queued/i)
})

it('names one queued import, singular, in the future tense', () => {
  const confirmation = switchConfirmation('paths', 1, 'switch')
  expect(confirmation.description).toContain('1 queued import will be discarded')
})

it('names many queued imports, plural', () => {
  const confirmation = switchConfirmation('paths', 3, 'switch')
  expect(confirmation.description).toContain('3 queued imports will be discarded')
})

it('words a switch and a close differently', () => {
  const switching = switchConfirmation('paths', 0, 'switch')
  const closing = switchConfirmation('paths', 0, 'close')
  expect(switching.title).not.toBe(closing.title)
  expect(switching.confirmLabel).not.toBe(closing.confirmLabel)
  expect(closing.title.toLowerCase()).toContain('close')
  expect(switching.title.toLowerCase()).toContain('switch')
})
