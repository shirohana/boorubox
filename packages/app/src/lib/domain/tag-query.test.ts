import { describe, expect, it } from 'vitest'
import {
  addTagToQuery,
  excludeTagFromQuery,
  parseTagSearch,
  removeTagFromQuery,
  toggleRatingInQuery,
  toggleTagInQuery,
} from './tag-utils'

describe('removeTagFromQuery', () => {
  it('removes tag with following "or"', () => {
    expect(removeTagFromQuery('cat or girl', 'cat')).toBe('girl')
  })

  it('removes tag with preceding "or"', () => {
    expect(removeTagFromQuery('girl or cat', 'cat')).toBe('girl')
  })

  it('removes tag without affecting others', () => {
    expect(removeTagFromQuery('dog cat girl', 'cat')).toBe('dog girl')
  })

  it('handles tag at beginning', () => {
    expect(removeTagFromQuery('cat girl', 'cat')).toBe('girl')
  })

  it('handles tag at end', () => {
    expect(removeTagFromQuery('girl cat', 'cat')).toBe('girl')
  })

  it('handles only tag', () => {
    expect(removeTagFromQuery('cat', 'cat')).toBe('')
  })

  it('preserves "or" between other tags when removing middle tag', () => {
    expect(removeTagFromQuery('dog or cat or girl', 'cat')).toBe('dog or girl')
  })

  it('handles case-insensitive "or" operator', () => {
    expect(removeTagFromQuery('cat OR girl', 'cat')).toBe('girl')
    expect(removeTagFromQuery('cat Or girl', 'cat')).toBe('girl')
  })

  it('does not remove tag if not present', () => {
    expect(removeTagFromQuery('dog girl', 'cat')).toBe('dog girl')
  })

  it('handles multiple spaces', () => {
    expect(removeTagFromQuery('dog  cat  girl', 'cat')).toBe('dog girl')
  })

  it('handles tag appearing multiple times', () => {
    expect(removeTagFromQuery('cat dog cat', 'cat')).toBe('dog')
  })
})

// Design D14: what these produce is a query string, so what they are asserted
// against is what the parser reads back out of it.

describe('addTagToQuery', () => {
  it('starts a query', () => {
    expect(addTagToQuery('', 'cat')).toBe('cat')
  })

  it('adds beside what is there', () => {
    expect(addTagToQuery('dog', 'cat')).toBe('dog cat')
  })

  it('leaves a tag the query already includes alone', () => {
    expect(addTagToQuery('cat dog', 'cat')).toBe('cat dog')
    expect(addTagToQuery('cat or dog', 'cat')).toBe('cat or dog')
  })

  it('stops excluding a tag it is asked to include', () => {
    const query = addTagToQuery('dog -cat', 'cat')
    expect(parseTagSearch(query).includeTags).toEqual(['dog', 'cat'])
    expect(parseTagSearch(query).excludeTags).toEqual([])
  })

  it('leaves the rest of the query intact', () => {
    const query = addTagToQuery('cat rating:s is:png', 'dog')
    const parsed = parseTagSearch(query)
    expect(parsed.includeTags).toEqual(['cat', 'dog'])
    expect(parsed.ratings).toEqual(['s'])
    expect(parsed.fileTypes).toEqual(['image/png'])
  })
})

describe('excludeTagFromQuery', () => {
  it('adds an exclusion', () => {
    expect(excludeTagFromQuery('cat', 'dog')).toBe('cat -dog')
  })

  it('leaves a tag the query already excludes alone', () => {
    expect(excludeTagFromQuery('cat -dog', 'dog')).toBe('cat -dog')
  })

  it('stops including a tag it is asked to exclude', () => {
    const query = excludeTagFromQuery('cat dog', 'cat')
    const parsed = parseTagSearch(query)
    expect(parsed.includeTags).toEqual(['dog'])
    expect(parsed.excludeTags).toEqual(['cat'])
  })
})

describe('toggleTagInQuery', () => {
  it('adds a tag the query does not name', () => {
    expect(toggleTagInQuery('dog', 'cat')).toBe('dog cat')
  })

  it('removes a tag the query includes', () => {
    expect(toggleTagInQuery('cat dog', 'cat')).toBe('dog')
  })

  it('removes a tag the query includes through an or group', () => {
    expect(parseTagSearch(toggleTagInQuery('cat or dog', 'cat')).includeTags).toEqual(['dog'])
  })

  it('removes a tag the query excludes', () => {
    const query = toggleTagInQuery('cat -dog', 'dog')
    expect(parseTagSearch(query).excludeTags).toEqual([])
    expect(parseTagSearch(query).includeTags).toEqual(['cat'])
  })

  it('leaves the rest of the query intact', () => {
    const parsed = parseTagSearch(toggleTagInQuery('cat rating:s is:png', 'cat'))
    expect(parsed.includeTags).toEqual([])
    expect(parsed.ratings).toEqual(['s'])
    expect(parsed.fileTypes).toEqual(['image/png'])
  })
})

describe('toggleRatingInQuery', () => {
  it('asks for a rating nothing asked for', () => {
    expect(toggleRatingInQuery('cat', 's')).toBe('cat rating:s')
  })

  it('rewrites the whole metatag when a second rating joins', () => {
    const query = toggleRatingInQuery('cat rating:s', 'q')
    expect(query).toBe('cat rating:q,s')
    expect(parseTagSearch(query).ratings).toEqual(['q', 's'])
  })

  it('rewrites the whole metatag when one rating leaves', () => {
    const query = toggleRatingInQuery('cat rating:q,s', 's')
    expect(query).toBe('cat rating:q')
    expect(parseTagSearch(query).ratings).toEqual(['q'])
  })

  it('drops the metatag with the last rating', () => {
    expect(toggleRatingInQuery('cat rating:s', 's')).toBe('cat')
  })

  it('reads a spelled-out rating and writes back the letter', () => {
    expect(toggleRatingInQuery('rating:explicit', 'g')).toBe('rating:e,g')
  })

  it('adds and removes is:unrated', () => {
    const added = toggleRatingInQuery('cat', 'unrated')
    expect(added).toBe('cat is:unrated')
    expect(parseTagSearch(added).includeUnrated).toBe(true)

    const removed = toggleRatingInQuery(added, 'unrated')
    expect(removed).toBe('cat')
    expect(parseTagSearch(removed).includeUnrated).toBe(false)
  })

  it('leaves unrated alone while a rating is toggled', () => {
    const parsed = parseTagSearch(toggleRatingInQuery('cat is:unrated rating:s', 'q'))
    expect(parsed.includeUnrated).toBe(true)
    expect(parsed.ratings).toEqual(['q', 's'])
    expect(parsed.includeTags).toEqual(['cat'])
  })
})
