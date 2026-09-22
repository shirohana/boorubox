import type { TagCategory } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { CATEGORY_TEXT_CLASS } from './categories'

const ALL_CATEGORIES: TagCategory[] = ['artist', 'copyright', 'character', 'meta', 'general']

describe('CATEGORY_TEXT_CLASS', () => {
  it('gives general no class, so the ordinary text colour shows through', () => {
    expect(CATEGORY_TEXT_CLASS.general).toBe('')
  })

  it('gives every other category a class', () => {
    for (const category of ALL_CATEGORIES) {
      if (category === 'general') continue
      expect(CATEGORY_TEXT_CLASS[category]).not.toBe('')
    }
  })

  it('names every TagCategory exactly once', () => {
    expect(Object.keys(CATEGORY_TEXT_CLASS).sort()).toEqual([...ALL_CATEGORIES].sort())
  })
})
