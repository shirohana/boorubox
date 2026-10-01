import type { RuleListEntry } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { filterRules } from './filter'

function entry(id: string, name: string, pattern: string, tags: string[]): RuleListEntry {
  return { rule: { id, name, pattern, tags } } as RuleListEntry
}

const entries = [
  entry('1', 'Pixiv sketches', 'pixiv', ['sketch']),
  entry('2', 'Twitter', 'twitter', ['twitter', 'lowres']),
  entry('3', 'Kantoku', 'kantoku', ['artist_a']),
]

describe('filterRules', () => {
  it('answers the same array for a blank or whitespace-only query', () => {
    expect(filterRules(entries, '')).toBe(entries)
    expect(filterRules(entries, '   ')).toBe(entries)
  })

  it('matches by name', () => {
    expect(filterRules(entries, 'sketches').map((e) => e.rule.id)).toEqual(['1'])
  })

  it('matches by pattern', () => {
    expect(filterRules(entries, 'tok').map((e) => e.rule.id)).toEqual(['3'])
  })

  it('matches by a tag', () => {
    expect(filterRules(entries, 'lowres').map((e) => e.rule.id)).toEqual(['2'])
  })

  it('folds case on both sides', () => {
    expect(filterRules(entries, 'PIXIV').map((e) => e.rule.id)).toEqual(['1'])
    expect(filterRules([entry('4', 'Mixed', 'X', ['LowRes'])], 'lowres')).toHaveLength(1)
  })

  it('answers an empty array when nothing matches', () => {
    expect(filterRules(entries, 'zzz')).toEqual([])
  })
})
