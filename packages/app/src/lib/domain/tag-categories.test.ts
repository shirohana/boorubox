import type { TagCategory } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { activeTerms } from './tag-utils'
import { CATEGORY_ORDER, categoryLabel, sidebarRows } from './tag-categories'

const ALL_CATEGORIES: TagCategory[] = ['artist', 'copyright', 'character', 'meta', 'general']

describe('CATEGORY_ORDER', () => {
  it('names every TagCategory exactly once', () => {
    expect(new Set(CATEGORY_ORDER)).toEqual(new Set(ALL_CATEGORIES))
    expect(CATEGORY_ORDER.length).toBe(ALL_CATEGORIES.length)
  })

  it('is design D1\'s fixed order: artist, copyright, character, general, meta', () => {
    expect(CATEGORY_ORDER).toEqual(['artist', 'copyright', 'character', 'general', 'meta'])
  })
})

describe('categoryLabel', () => {
  it('capitalises the category name', () => {
    expect(categoryLabel('artist')).toBe('Artist')
    expect(categoryLabel('copyright')).toBe('Copyright')
    expect(categoryLabel('character')).toBe('Character')
    expect(categoryLabel('meta')).toBe('Meta')
    expect(categoryLabel('general')).toBe('General')
  })
})

describe('sidebarRows', () => {
  interface Row { name: string }
  const row = (name: string): Row => ({ name })
  const nameOf = (row: Row) => row.name

  // Spec `tag-sidebar`, "Order": kantoku (artist), azur_lane (copyright) and
  // highres (meta) each name their category; every other name is general.
  const categoryOf = (name: string): TagCategory => {
    if (name === 'kantoku') return 'artist'
    if (name === 'azur_lane') return 'copyright'
    if (name === 'highres') return 'meta'
    return 'general'
  }

  it('nothing hidden keeps today\'s order', () => {
    const rows = ['1girl', 'kantoku', 'azur_lane', 'highres', 'cat'].map(row)
    const isActive = (name: string) => name === 'highres'

    const result = sidebarRows(rows, nameOf, isActive, categoryOf, new Set())

    expect(result.map(nameOf)).toEqual(['highres', 'kantoku', 'azur_lane', '1girl', 'cat'])
  })

  it('a hidden category\'s inactive rows are left out', () => {
    const rows = ['kantoku', '1girl'].map(row)

    const result = sidebarRows(rows, nameOf, () => false, categoryOf, new Set(['artist']))

    expect(result.map(nameOf)).toEqual(['1girl'])
  })

  it('a hidden category\'s active row stays, first', () => {
    const rows = ['1girl', 'kantoku'].map(row)
    const isActive = (name: string) => name === 'kantoku'

    const result = sidebarRows(rows, nameOf, isActive, categoryOf, new Set(['artist']))

    expect(result.map(nameOf)).toEqual(['kantoku', '1girl'])
  })

  it('an or-group term counts as active', () => {
    const rows = ['kantoku', '1girl', 'cat'].map(row)
    const terms = activeTerms('kantoku or 1girl')
    const isActive = (name: string) => terms.included.has(name) || terms.excluded.has(name)

    const result = sidebarRows(rows, nameOf, isActive, categoryOf, new Set())

    expect(result.map(nameOf)).toEqual(['kantoku', '1girl', 'cat'])
  })

  it('every category hidden leaves only the active rows', () => {
    const rows = ['kantoku', '1girl', 'cat'].map(row)
    const isActive = (name: string) => name === 'cat'

    const result = sidebarRows(rows, nameOf, isActive, categoryOf, new Set(CATEGORY_ORDER))

    expect(result.map(nameOf)).toEqual(['cat'])
  })

  it('every category hidden with nothing active is empty', () => {
    const rows = ['kantoku', '1girl'].map(row)

    const result = sidebarRows(rows, nameOf, () => false, categoryOf, new Set(CATEGORY_ORDER))

    expect(result).toEqual([])
  })
})
