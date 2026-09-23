import type { TagEditSpec } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { confirmedCount, confirmPrompt, needsConfirmation } from './pending-write'

const NO_COLLECTIONS = { addCollections: [], removeCollections: [] }

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

  it('counts the ids of an edit write, the same rule as every other kind', () => {
    const spec: TagEditSpec = { add: ['tagme'], remove: [], addCollections: [], removeCollections: [] }
    expect(confirmedCount({ kind: 'edit', ids: ['a', 'b', 'c'], spec, label: 'tagme' }, 0)).toBe(3)
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

  it('names the tag being added and the count, not destructive', () => {
    const spec: TagEditSpec = { add: ['tagme'], remove: [], ...NO_COLLECTIONS }
    const prompt = confirmPrompt({ kind: 'edit', ids: ['a', 'b'], spec, label: 'tagme' }, 0)
    expect(prompt.title).toBe('Add “tagme” to 2 images?')
    expect(prompt.confirmLabel).toBe('Add tag')
    expect(prompt.destructive).toBe(false)
  })

  it('names the tag being removed and the count, not destructive', () => {
    const spec: TagEditSpec = { add: [], remove: ['tagme'], ...NO_COLLECTIONS }
    const prompt = confirmPrompt({ kind: 'edit', ids: ['a', 'b'], spec, label: 'tagme' }, 0)
    expect(prompt.title).toBe('Remove “tagme” from 2 images?')
    expect(prompt.confirmLabel).toBe('Remove tag')
    expect(prompt.destructive).toBe(false)
  })

  it('names the stamp by its label and says there is no undo, once no single tag reads', () => {
    const spec: TagEditSpec = { add: ['cat', 'animal'], remove: [], ...NO_COLLECTIONS }
    const prompt = confirmPrompt({ kind: 'edit', ids: ['a', 'b'], spec, label: 'Cat' }, 0)
    expect(prompt.title).toBe('Apply “Cat” to 2 images?')
    expect(prompt.confirmLabel).toBe('Apply')
    expect(prompt.destructive).toBe(false)
    expect(prompt.description).toContain('no undo')
  })

  it('reads a collection-only or rated edit as a stamp apply, not a single tag', () => {
    const collectionOnly: TagEditSpec = { add: [], remove: [], addCollections: ['cute'], removeCollections: [] }
    expect(confirmPrompt({ kind: 'edit', ids: ['a'], spec: collectionOnly, label: 'Cute' }, 0).title)
      .toBe('Apply “Cute” to 1 image?')

    const rated: TagEditSpec = { add: ['tagme'], remove: [], ...NO_COLLECTIONS, rating: 'g' }
    expect(confirmPrompt({ kind: 'edit', ids: ['a'], spec: rated, label: 'Reviewed' }, 0).title)
      .toBe('Apply “Reviewed” to 1 image?')
  })
})
