import type { ArtistEntry } from '@boorubox/shared'
import { expect, it } from 'vitest'
import { filterArtists } from './artist-filter'

const entries: ArtistEntry[] = [
  { tag: 'metaljelly', urls: ['x.com/metaljelly0811'] },
  { tag: 'alice', urls: ['x.com/alice'] },
  { tag: 'bob_art', urls: ['x.com/bobart'] },
]

it('returns every entry in order for an empty or all-space query', () => {
  expect(filterArtists(entries, '')).toEqual(entries)
  expect(filterArtists(entries, '   ')).toEqual(entries)
})

it('finds a tag match ignoring case', () => {
  expect(filterArtists(entries, 'BOB')).toEqual([entries[2]])
})

it('finds a URL match', () => {
  expect(filterArtists(entries, '0811')).toEqual([entries[0]])
})

it('finds nothing for text no entry contains', () => {
  expect(filterArtists(entries, 'zzz')).toEqual([])
})
