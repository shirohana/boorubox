import type { BooruSite, PostRef } from '@boorubox/shared'
import { expect, it } from 'vitest'
import { postedEntries, postedNames, postUrl, siteUrl, unpostedSites } from './posted'

function site(overrides: Partial<BooruSite> = {}): BooruSite {
  return {
    id: 'danbooru-donmai-us',
    name: 'Danbooru',
    baseUrl: 'https://danbooru.donmai.us',
    username: 'alice',
    createdAt: 0,
    updatedAt: 0,
    ...overrides,
  }
}

function post(overrides: Partial<PostRef> = {}): PostRef {
  return { site: 'danbooru-donmai-us', remoteId: '555', postedAt: 0, ...overrides }
}

it('joins a path to the site root, without doubling the separator', () => {
  expect(siteUrl('https://booru.lan', 'uploads/7')).toBe('https://booru.lan/uploads/7')
  expect(siteUrl('https://booru.lan//', 'uploads/7')).toBe('https://booru.lan/uploads/7')
})

it('builds the post address from the site, without doubling the separator', () => {
  expect(postUrl('https://danbooru.donmai.us', '555')).toBe('https://danbooru.donmai.us/posts/555')
  expect(postUrl('https://booru.lan/', '9')).toBe('https://booru.lan/posts/9')
})

it('names every post for the tile, falling back to the key of a removed site', () => {
  expect(postedNames([post(), post({ site: 'booru-lan', remoteId: '9' })], [site()])).toEqual([
    'Posted to Danbooru #555',
    'Posted to booru-lan #9',
  ])
})

it('names and links every site an image was posted to', () => {
  const entries = postedEntries(
    [post(), post({ site: 'booru-lan', remoteId: '9' })],
    [site(), site({ id: 'booru-lan', name: 'Home booru', baseUrl: 'http://booru.lan' })],
  )

  expect(entries.map((entry) => [entry.name, entry.url])).toEqual([
    ['Danbooru', 'https://danbooru.donmai.us/posts/555'],
    ['Home booru', 'http://booru.lan/posts/9'],
  ])
})

it('keeps a record whose site is no longer configured, showing its key and no link', () => {
  expect(postedEntries([post()], [])).toEqual([
    { key: 'danbooru-donmai-us', name: 'danbooru-donmai-us', remoteId: '555', postedAt: 0, url: null },
  ])
})

it('offers only the sites the image is not already on', () => {
  const sites = [site(), site({ id: 'booru-lan', name: 'Home booru' })]

  expect(unpostedSites([post()], sites).map((left) => left.id)).toEqual(['booru-lan'])
  expect(unpostedSites([], sites)).toHaveLength(2)
  expect(unpostedSites([post(), post({ site: 'booru-lan' })], sites)).toHaveLength(0)
})
