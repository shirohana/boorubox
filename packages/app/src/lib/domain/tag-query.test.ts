import { describe, expect, it } from 'vitest'
import {
  activeTerms,
  addTagToQuery,
  excludeTagFromQuery,
  parseTagSearch,
  removeTagFromQuery,
  tagList,
  toggleAccountInQuery,
  toggleCollectionInQuery,
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

  it('round-trips a slug the parser must not cut short', () => {
    const query = toggleCollectionInQuery('cat', 'to-upload')
    expect(query).toBe('cat collection:to-upload')
    expect(parseTagSearch(query).collections).toEqual(['to-upload'])
    expect(toggleCollectionInQuery(query, 'to-upload')).toBe('cat')
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

describe('toggleCollectionInQuery', () => {
  it('adds a collection the query does not name', () => {
    expect(toggleCollectionInQuery('cat', 'favorites')).toBe('cat collection:favorites')
  })

  it('removes a collection the query already names', () => {
    expect(toggleCollectionInQuery('cat collection:favorites', 'favorites')).toBe('cat')
  })

  it('rewrites the whole metatag when a second collection joins', () => {
    const query = toggleCollectionInQuery('collection:favorites', 'queue')
    expect(query).toBe('collection:favorites,queue')
    expect(parseTagSearch(query).collections).toEqual(['favorites', 'queue'])
  })

  it('leaves an exclusion untouched and adds beside it', () => {
    expect(toggleCollectionInQuery('-collection:queue', 'favorites'))
      .toBe('-collection:queue collection:favorites')
  })

  it('takes the exclusion out instead of contradicting it', () => {
    expect(toggleCollectionInQuery('cat -collection:favorites', 'favorites')).toBe('cat')
  })

  it('leaves the rest of the query intact', () => {
    const parsed = parseTagSearch(toggleCollectionInQuery('cat rating:s', 'favorites'))
    expect(parsed.collections).toEqual(['favorites'])
    expect(parsed.includeTags).toEqual(['cat'])
    expect(parsed.ratings).toEqual(['s'])
  })
})

describe('activeTerms', () => {
  it('reads included, excluded and or-grouped tags, plus accounts and collections', () => {
    const terms = activeTerms(
      'cat dog or bird -mouse account:alice -account:eve collection:favorites -collection:queue',
    )
    expect(terms.included).toEqual(new Set(['cat', 'dog', 'bird']))
    expect(terms.excluded).toEqual(new Set(['mouse']))
    expect(terms.accounts).toEqual(new Set(['alice']))
    expect(terms.excludedAccounts).toEqual(new Set(['eve']))
    expect(terms.collections).toEqual(new Set(['favorites']))
    expect(terms.excludedCollections).toEqual(new Set(['queue']))
  })

  it('answers empty sets for an empty query', () => {
    const terms = activeTerms('')
    expect(terms.included.size).toBe(0)
    expect(terms.excluded.size).toBe(0)
    expect(terms.accounts.size).toBe(0)
    expect(terms.excludedAccounts.size).toBe(0)
    expect(terms.collections.size).toBe(0)
    expect(terms.excludedCollections.size).toBe(0)
  })
})

describe('toggleAccountInQuery', () => {
  it('starts a query', () => {
    expect(toggleAccountInQuery('', 'alice')).toBe('account:alice')
  })

  it('removes the only account, leaving the rest of the query', () => {
    expect(toggleAccountInQuery('cat account:alice', 'alice')).toBe('cat')
  })

  it('rewrites the whole list when one account leaves it', () => {
    expect(toggleAccountInQuery('account:alice,bob', 'alice')).toBe('account:bob')
  })

  it('leaves an exclusion untouched and adds beside it', () => {
    expect(toggleAccountInQuery('-account:eve', 'alice')).toBe('-account:eve account:alice')
  })

  it('takes the exclusion out instead of contradicting it', () => {
    expect(toggleAccountInQuery('cat -account:alice', 'alice')).toBe('cat')
    expect(toggleAccountInQuery('-account:alice,eve', 'alice')).toBe('-account:eve')
  })

  it('keeps the case it is given', () => {
    expect(toggleAccountInQuery('', 'Bob')).toBe('account:Bob')
    expect(toggleAccountInQuery('account:Bob', 'Bob')).toBe('')
  })
})

describe('tagList', () => {
  it('splits on any run of whitespace and drops the empties', () => {
    expect(tagList('  cat   dog\n')).toEqual(['cat', 'dog'])
    expect(tagList('   ')).toEqual([])
  })

  it('keeps every token literally, query syntax included', () => {
    expect(tagList('-foo rating:s a or b')).toEqual(['-foo', 'rating:s', 'a', 'or', 'b'])
  })
})
