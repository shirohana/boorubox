// @vitest-environment jsdom
// @vitest-environment-options { "url": "https://page.test/gallery" }

import { expect, it } from 'vitest'

import { extractPageContext, findAdapter, type SiteAdapter } from './index.js'

const stub: SiteAdapter = {
  site: 'stub',
  hosts: ['stub.test'],
  extract: () => ({ handle: 'alice', aliases: ['a', 'b'] }),
}

function contextOn(hostname: string) {
  return {
    document,
    hostname,
    imageUrl: 'https://cdn.test/a.png',
    pageUrl: `https://${hostname}/p/1`,
  }
}

it('matches a host and its subdomains, and nothing else', () => {
  expect(findAdapter('stub.test', [stub])).toBe(stub)
  expect(findAdapter('www.stub.test', [stub])).toBe(stub)
  expect(findAdapter('notstub.test', [stub])).toBeUndefined()
  expect(findAdapter('stub.test.evil.example', [stub])).toBeUndefined()
})

function recordOn(hostname: string, adapters: SiteAdapter[]) {
  return extractPageContext(contextOn(hostname), adapters).record
}

it('sends the site and the fields it read', () => {
  const record = recordOn('stub.test', [stub])

  expect(record).toEqual({ site: 'stub', fields: { handle: 'alice', aliases: ['a', 'b'] } })
})

it('sends no record for a site no adapter recognises', () => {
  expect(recordOn('unknown.test', [stub])).toBeNull()
})

it('sends no record when the adapter raises, rather than propagating', () => {
  const raising: SiteAdapter = {
    site: 'stub',
    hosts: ['stub.test'],
    extract: () => {
      throw new Error('the markup moved')
    },
  }

  expect(recordOn('stub.test', [raising])).toBeNull()
})

it('drops the fields the page did not have and keeps the rest', () => {
  const partial: SiteAdapter = {
    site: 'stub',
    hosts: ['stub.test'],
    extract: () => ({
      handle: 'alice',
      postText: '',
      title: null,
      artist: undefined,
      aliases: [],
    }),
  }

  const record = recordOn('stub.test', [partial])

  expect(record).toEqual({ site: 'stub', fields: { handle: 'alice' } })
})

it('keeps the site when nothing on the page could be read', () => {
  const blind: SiteAdapter = { site: 'stub', hosts: ['stub.test'], extract: () => ({}) }

  expect(recordOn('stub.test', [blind])).toEqual({ site: 'stub', fields: {} })
})

it('titles the page with the live document title when the adapter has none', () => {
  document.title = 'The gallery, as it is right now'

  expect(extractPageContext(contextOn('stub.test'), [stub]).pageTitle)
    .toBe('The gallery, as it is right now')
})

it('lets the adapter name the page, and falls back when it cannot', () => {
  document.title = 'a stale tab title'
  const named: SiteAdapter = { ...stub, title: (fields) => `post by ${fields.handle}` }
  const silent: SiteAdapter = { ...stub, title: () => undefined }

  expect(extractPageContext(contextOn('stub.test'), [named]).pageTitle).toBe('post by alice')
  expect(extractPageContext(contextOn('stub.test'), [silent]).pageTitle).toBe('a stale tab title')
})

it('keeps the record when the adapter raises while naming the page', () => {
  document.title = 'a stale tab title'
  const raising: SiteAdapter = {
    ...stub,
    title: () => {
      throw new Error('the markup moved')
    },
  }

  const page = extractPageContext(contextOn('stub.test'), [raising])

  expect(page.pageTitle).toBe('a stale tab title')
  expect(page.record).toEqual({ site: 'stub', fields: { handle: 'alice', aliases: ['a', 'b'] } })
})
