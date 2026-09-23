import type { TagCategory } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { CATEGORY_ICON, CATEGORY_ORDER, CATEGORY_TEXT_CLASS, searchMark, searchMarkClass, SEARCH_MARK_CLASS } from './categories'

const ALL_CATEGORIES: TagCategory[] = ['artist', 'copyright', 'character', 'meta', 'general']

describe('CATEGORY_TEXT_CLASS', () => {
  it('gives general the blue class (design D2)', () => {
    expect(CATEGORY_TEXT_CLASS.general).toBe('text-blue-600 dark:text-blue-400')
  })

  it('gives every category a class', () => {
    for (const category of ALL_CATEGORIES) {
      expect(CATEGORY_TEXT_CLASS[category]).not.toBe('')
    }
  })

  it('names every TagCategory exactly once', () => {
    expect(Object.keys(CATEGORY_TEXT_CLASS).sort()).toEqual([...ALL_CATEGORIES].sort())
  })
})

describe('CATEGORY_ICON', () => {
  it('names every CATEGORY_ORDER entry exactly once', () => {
    expect(Object.keys(CATEGORY_ICON).sort()).toEqual([...CATEGORY_ORDER].sort())
  })
})

describe('searchMark', () => {
  const included = new Set(['cat', 'alice'])
  const excluded = new Set(['dog', 'bob'])

  it('marks a tag included', () => {
    expect(searchMark('cat', included, excluded)).toBe('included')
  })

  it('marks a tag excluded', () => {
    expect(searchMark('dog', included, excluded)).toBe('excluded')
  })

  it('marks a tag neither', () => {
    expect(searchMark('bird', included, excluded)).toBe('none')
  })

  it('marks an account included', () => {
    expect(searchMark('alice', included, excluded)).toBe('included')
  })

  it('marks an account excluded', () => {
    expect(searchMark('bob', included, excluded)).toBe('excluded')
  })

  it('marks an account neither', () => {
    expect(searchMark('carol', included, excluded)).toBe('none')
  })
})

describe('SEARCH_MARK_CLASS', () => {
  it('is empty for none, and visible for included and excluded', () => {
    expect(SEARCH_MARK_CLASS.none).toBe('')
    expect(SEARCH_MARK_CLASS.included).not.toBe('')
    expect(SEARCH_MARK_CLASS.excluded).not.toBe('')
  })

  it('is background only, no underline or strike-through (owner, 2026-09-23)', () => {
    expect(SEARCH_MARK_CLASS.included).not.toMatch(/underline|line-through/)
    expect(SEARCH_MARK_CLASS.excluded).not.toMatch(/underline|line-through/)
  })

  it('gives included the extra weight and leaves excluded and none at the base weight', () => {
    expect(SEARCH_MARK_CLASS.included).toMatch(/font-medium/)
    expect(SEARCH_MARK_CLASS.excluded).not.toMatch(/font-medium/)
    expect(SEARCH_MARK_CLASS.none).not.toMatch(/font-medium/)
  })

  it('carries its own hover shade for included and excluded, darker than the base tint', () => {
    expect(SEARCH_MARK_CLASS.included).toMatch(/hover:bg-emerald-500\/25/)
    expect(SEARCH_MARK_CLASS.excluded).toMatch(/hover:bg-destructive\/20/)
  })
})

describe('searchMarkClass', () => {
  it('returns the neutral hover for none, not the mark table', () => {
    expect(searchMarkClass('none', 'hover:bg-accent')).toBe('hover:bg-accent')
  })

  it('returns the mark class for included and excluded, never the neutral hover', () => {
    expect(searchMarkClass('included', 'hover:bg-accent')).toBe(SEARCH_MARK_CLASS.included)
    expect(searchMarkClass('excluded', 'hover:bg-accent')).toBe(SEARCH_MARK_CLASS.excluded)
    expect(searchMarkClass('included', 'hover:bg-accent')).not.toMatch(/hover:bg-accent/)
    expect(searchMarkClass('excluded', 'hover:bg-accent')).not.toMatch(/hover:bg-accent/)
  })
})
