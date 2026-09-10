import { describe, expect, it } from 'vitest'
import { confirmedCount, confirmPrompt, needsTrashConfirmation } from './trash-actions'

describe('needsTrashConfirmation', () => {
  it('lets one image go without a dialog', () => {
    expect(needsTrashConfirmation(1)).toBe(false)
  })

  it('asks from two upwards', () => {
    expect(needsTrashConfirmation(2)).toBe(true)
    expect(needsTrashConfirmation(5000)).toBe(true)
  })
})

describe('confirmedCount', () => {
  it('counts the ids a pending write names', () => {
    expect(confirmedCount({ kind: 'trash', ids: ['a', 'b'] }, 0)).toBe(2)
    expect(confirmedCount({ kind: 'delete', ids: ['a'] }, 0)).toBe(1)
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
})
