import type { TagCategory } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { CATEGORY_ORDER, categoryLabel } from './tag-categories'

const ALL_CATEGORIES: TagCategory[] = ['artist', 'copyright', 'character', 'meta', 'general']

describe('CATEGORY_ORDER', () => {
  it('names every TagCategory exactly once', () => {
    expect(new Set(CATEGORY_ORDER)).toEqual(new Set(ALL_CATEGORIES))
    expect(CATEGORY_ORDER.length).toBe(ALL_CATEGORIES.length)
  })

  it('is design D6\'s fixed order: artist, copyright, character, meta, general', () => {
    expect(CATEGORY_ORDER).toEqual(['artist', 'copyright', 'character', 'meta', 'general'])
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
