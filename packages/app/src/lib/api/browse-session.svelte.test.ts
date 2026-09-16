// @vitest-environment jsdom

import { expect, it } from 'vitest'
import { TRASH_DEFAULT_SORT } from './search.svelte'
import { BrowseSession } from './browse-session.svelte'

it('answers the same instance for two asks of the same view', () => {
  const session = new BrowseSession()

  expect(session.resultsFor('library')).toBe(session.resultsFor('library'))
})

it('answers two different instances for the library and the trash', () => {
  const session = new BrowseSession()

  expect(session.resultsFor('library')).not.toBe(session.resultsFor('trash'))
})

it('gives the trash instance the trash default sort', () => {
  const session = new BrowseSession()

  expect(session.resultsFor('trash').sort).toEqual(TRASH_DEFAULT_SORT)
})
