import { describe, expect, it } from 'vitest'
import { parseStamp } from './stamp'

describe('parseStamp', () => {
  it('reads tags both ways', () => {
    expect(parseStamp('cat animal -dog')).toEqual({
      edit: { add: ['cat', 'animal'], remove: ['dog'], addCollections: [], removeCollections: [] },
    })
  })

  it('reads collection moves, lower-cased', () => {
    expect(parseStamp('collection:cute -collection:uncategorized')).toEqual({
      edit: {
        add: [],
        remove: [],
        addCollections: ['cute'],
        removeCollections: ['uncategorized'],
      },
    })
    expect(parseStamp('Collection:Cute')).toEqual({
      edit: { add: [], remove: [], addCollections: ['cute'], removeCollections: [] },
    })
  })

  it('sets the rating, the last one winning', () => {
    expect(parseStamp('rating:g')).toEqual({
      edit: { add: [], remove: [], addCollections: [], removeCollections: [], rating: 'g' },
    })
    expect(parseStamp('rating:s rating:g')).toEqual({
      edit: { add: [], remove: [], addCollections: [], removeCollections: [], rating: 'g' },
    })
  })

  it('refuses -rating: as not an edit', () => {
    expect(parseStamp('-rating:g')).toEqual({ error: '“-rating:g” is not an edit.' })
  })

  it('names a search-only metatag as not an edit', () => {
    expect(parseStamp('cat is:png')).toEqual({ error: '“is:png” is not an edit.' })
    expect(parseStamp('tagcount:5')).toEqual({ error: '“tagcount:5” is not an edit.' })
    expect(parseStamp('account:alice')).toEqual({ error: '“account:alice” is not an edit.' })
    expect(parseStamp('posted:danbooru')).toEqual({ error: '“posted:danbooru” is not an edit.' })
  })

  it('names or as not an edit', () => {
    expect(parseStamp('cat or dog')).toEqual({ error: '“or” is not an edit.' })
  })

  it('keeps a category prefix in add, verbatim', () => {
    expect(parseStamp('artist:kantoku')).toEqual({
      edit: { add: ['artist:kantoku'], remove: [], addCollections: [], removeCollections: [] },
    })
  })

  it('refuses a tag that is both added and removed', () => {
    expect(parseStamp('cat -cat')).toEqual({ error: '“cat” is both added and removed.' })
  })

  it('refuses an empty text', () => {
    expect(parseStamp('')).toEqual({ error: 'A stamp needs at least one token.' })
    expect(parseStamp('   ')).toEqual({ error: 'A stamp needs at least one token.' })
  })

  it('refuses a prefix with nothing after it, naming the token', () => {
    for (const token of ['collection:', '-collection:', '-']) {
      const result = parseStamp(`cat ${token}`)
      expect('error' in result && result.error).toContain(token)
    }
  })
  it('drops duplicates within one list', () => {
    expect(parseStamp('cat cat -dog -dog')).toEqual({
      edit: { add: ['cat'], remove: ['dog'], addCollections: [], removeCollections: [] },
    })
  })

  it('refuses an unknown rating letter', () => {
    expect(parseStamp('rating:x')).toEqual({ error: '“rating:x” is not an edit.' })
  })
})
