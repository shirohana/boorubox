import { describe, expect, it } from 'vitest'
import {
  getVisualOrder,
  getXAccountFromUrl,
  groupImagesByDuplicates,
  groupImagesByXAccount,
} from './grouping'
import { img } from './image-fixture'

describe('getXAccountFromUrl', () => {
  it('extracts account from x.com status URL', () => {
    expect(getXAccountFromUrl('https://x.com/alice/status/123')).toBe('alice')
  })

  it('extracts account from twitter.com and www variants', () => {
    expect(getXAccountFromUrl('https://twitter.com/bob/status/1')).toBe('bob')
    expect(getXAccountFromUrl('https://www.x.com/carol')).toBe('carol')
    expect(getXAccountFromUrl('https://www.twitter.com/dave/photo')).toBe('dave')
  })

  it('returns null for non-account reserved paths', () => {
    for (const p of ['i', 'home', 'explore', 'notifications', 'messages', 'search']) {
      expect(getXAccountFromUrl(`https://x.com/${p}/status/1`)).toBeNull()
    }
  })

  it('returns null for non-x hosts and invalid URLs', () => {
    expect(getXAccountFromUrl('https://example.com/alice')).toBeNull()
    expect(getXAccountFromUrl('not a url')).toBeNull()
    expect(getXAccountFromUrl('https://x.com/')).toBeNull()
  })

  it('returns null for a record with no page URL', () => {
    expect(getXAccountFromUrl(null)).toBeNull()
  })
})

describe('groupImagesByXAccount', () => {
  it('groups images by account, dropping non-account images', () => {
    const images = [
      img({ id: '1', pageUrl: 'https://x.com/alice/status/1' }),
      img({ id: '2', pageUrl: 'https://x.com/alice/status/2' }),
      img({ id: '3', pageUrl: 'https://x.com/bob/status/3' }),
      img({ id: '4', pageUrl: 'https://example.com/page' }),
    ]
    const groups = groupImagesByXAccount(images)
    expect(groups.get('alice')!.map((i) => i.id)).toEqual(['1', '2'])
    expect(groups.get('bob')!.map((i) => i.id)).toEqual(['3'])
    expect(groups.has('example.com')).toBe(false)
    expect(groups.size).toBe(2)
  })
})

describe('groupImagesByDuplicates', () => {
  it('only returns groups with 2+ images sharing dimensions and size', () => {
    const images = [
      img({ id: '1', width: 100, height: 100, size: 500 }),
      img({ id: '2', width: 100, height: 100, size: 500 }),
      img({ id: '3', width: 200, height: 200, size: 999 }), // unique
      img({ id: '4', width: 100, height: 100, size: 600 }), // same dims, diff size
    ]
    const dups = groupImagesByDuplicates(images)
    expect(dups.size).toBe(1)
    expect(dups.get('100×100-500')!.map((i) => i.id)).toEqual(['1', '2'])
  })
})

describe('getVisualOrder', () => {
  const a1 = img({ id: 'a1', pageUrl: 'https://x.com/alice/status/1' })
  const a2 = img({ id: 'a2', pageUrl: 'https://x.com/alice/status/2' })
  const b1 = img({ id: 'b1', pageUrl: 'https://x.com/bob/status/1' })
  const filtered = [b1, a1, a2]

  it('returns filtered images unchanged when ungrouped', () => {
    expect(getVisualOrder(filtered, 'none')).toEqual(filtered)
  })

  it('orders by account count desc then alpha when grouped by x-account', () => {
    // alice has 2, bob has 1 -> alice group first
    const order = getVisualOrder(filtered, 'x-account').map((i) => i.id)
    expect(order).toEqual(['a1', 'a2', 'b1'])
  })

  it('orders duplicate groups by key alphabetically', () => {
    const images = [
      img({ id: 'big1', width: 200, height: 200, size: 10 }),
      img({ id: 'small1', width: 100, height: 100, size: 10 }),
      img({ id: 'small2', width: 100, height: 100, size: 10 }),
      img({ id: 'big2', width: 200, height: 200, size: 10 }),
    ]
    // keys: '100×100-10' < '200×200-10'
    const order = getVisualOrder(images, 'duplicates').map((i) => i.id)
    expect(order).toEqual(['small1', 'small2', 'big1', 'big2'])
  })
})
