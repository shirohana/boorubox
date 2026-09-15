import { describe, expect, it } from 'vitest'
import { confirmedCount, confirmPrompt, needsConfirmation } from './pending-write'

describe('needsConfirmation', () => {
  it('lets one image go without a dialog', () => {
    expect(needsConfirmation(1)).toBe(false)
  })

  it('asks from two upwards', () => {
    expect(needsConfirmation(2)).toBe(true)
    expect(needsConfirmation(5000)).toBe(true)
  })
})

describe('confirmedCount', () => {
  it('counts the ids a pending write names', () => {
    expect(confirmedCount({ kind: 'trash', ids: ['a', 'b'] }, 0)).toBe(2)
    expect(confirmedCount({ kind: 'delete', ids: ['a'] }, 0)).toBe(1)
    expect(confirmedCount({ kind: 'rate', ids: ['a', 'b', 'c'], rating: 'g' }, 0)).toBe(3)
  })

  it('reads the trash count for an empty, which names no ids', () => {
    expect(confirmedCount({ kind: 'empty' }, 37)).toBe(37)
  })
})

describe('confirmPrompt', () => {
  it('names the count and does not claim the trash is permanent', () => {
    const prompt = confirmPrompt({ kind: 'trash', ids: ['a', 'b', 'c'] }, 0)
    expect(prompt.title).toBe('Move 3 images to the trash?')
    expect(prompt.confirmLabel).toBe('Move to trash')
    expect(prompt.description).not.toContain('cannot be undone')
    expect(prompt.description).toContain('Restore')
  })

  it('says a permanent delete cannot be undone', () => {
    const prompt = confirmPrompt({ kind: 'delete', ids: ['a'] }, 0)
    expect(prompt.title).toBe('Permanently delete 1 image?')
    expect(prompt.confirmLabel).toBe('Delete forever')
    expect(prompt.description).toContain('cannot be undone')
  })

  it('asks about the whole trash when emptying it', () => {
    expect(confirmPrompt({ kind: 'empty' }, 1200).title)
      .toBe(`Permanently delete ${(1200).toLocaleString()} images?`)
  })

  it('names the count and the rating, capitalized, for a rating write', () => {
    const prompt = confirmPrompt({ kind: 'rate', ids: ['a', 'b'], rating: 's' }, 0)
    expect(prompt.title).toBe('Set 2 images to Sensitive?')
    expect(prompt.confirmLabel).toBe('Set rating')
    expect(prompt.destructive).toBe(true)
    expect(prompt.description).toContain('no undo')
  })

  it('names clearing the rating when the write sets no rating', () => {
    const prompt = confirmPrompt({ kind: 'rate', ids: ['a', 'b'], rating: null }, 0)
    expect(prompt.title).toBe('Clear the rating of 2 images?')
    expect(prompt.confirmLabel).toBe('Clear rating')
    expect(prompt.destructive).toBe(true)
  })
})
