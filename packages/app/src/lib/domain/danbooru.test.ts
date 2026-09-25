import type { TagCategory } from '@boorubox/shared'
import { expect, it } from 'vitest'
import { danbooruLookup } from './danbooru'

it('searches the artist by name, both bracketed params encoded', () => {
  expect(danbooruLookup('kantoku', 'artist')).toEqual({
    label: 'Search artist on Danbooru',
    url: 'https://danbooru.donmai.us/artists?commit=Search&search%5Bany_name_matches%5D=kantoku&search%5Border%5D=created_at',
  })
})

it('opens the wiki page for a tag of any category but artist', () => {
  const categories: TagCategory[] = ['copyright', 'character', 'general', 'meta']
  for (const category of categories) {
    expect(danbooruLookup('solo', category)).toEqual({
      label: 'Open Danbooru wiki',
      url: 'https://danbooru.donmai.us/wiki_pages/solo',
    })
  }
})

it('encodes a name containing a slash', () => {
  expect(danbooruLookup('rating/explicit', 'general').url).toBe(
    'https://danbooru.donmai.us/wiki_pages/rating%2Fexplicit',
  )
})

it('leaves parentheses in a tag name unescaped', () => {
  expect(danbooruLookup('tashkent_(azur_lane)', 'general').url).toBe(
    'https://danbooru.donmai.us/wiki_pages/tashkent_(azur_lane)',
  )
})

it('encodes a slash inside a tag name', () => {
  expect(danbooruLookup('fate/grand_order', 'general').url).toBe(
    'https://danbooru.donmai.us/wiki_pages/fate%2Fgrand_order',
  )
})

it('encodes a Japanese name', () => {
  expect(danbooruLookup('ずんだもん', 'general').url).toBe(
    'https://danbooru.donmai.us/wiki_pages/%E3%81%9A%E3%82%93%E3%81%A0%E3%82%82%E3%82%93',
  )
})
