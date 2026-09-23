import type { TagCountFilter } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { COUNT_METATAGS, parseTagSearch } from './tag-utils'

/** `ratings`, `fileTypes` and `accounts` were Sets before D13: order means nothing. */
const sorted = (values: string[]) => [...values].sort()

describe('parseTagSearch', () => {
  describe('empty and whitespace', () => {
    it('should handle empty string', () => {
      const result = parseTagSearch('')
      expect(result.includeTags).toEqual([])
      expect(result.excludeTags).toEqual([])
      expect(result.orGroups).toEqual([])
      expect(result.ratings).toEqual([])
      expect(result.fileTypes).toEqual([])
      expect(result.tagCountTerms).toEqual([])
      expect(result.includeUnrated).toBe(false)
      expect(result.accounts).toEqual([])
      expect(result.excludeAccounts).toEqual([])
      expect(result.collections).toEqual([])
      expect(result.excludeCollections).toEqual([])
    })

    it('should handle only whitespace', () => {
      const result = parseTagSearch('   ')
      expect(result.includeTags).toEqual([])
      expect(result.excludeTags).toEqual([])
    })

    it('should handle multiple spaces between tags', () => {
      const result = parseTagSearch('girl  cat')
      expect(result.includeTags).toEqual(['girl', 'cat'])
    })
  })

  describe('basic tag inclusion (AND)', () => {
    it('should parse single tag', () => {
      const result = parseTagSearch('girl')
      expect(result.includeTags).toEqual(['girl'])
      expect(result.excludeTags).toEqual([])
    })

    it('should parse multiple tags (AND logic)', () => {
      const result = parseTagSearch('girl cat dog')
      expect(result.includeTags).toEqual(['girl', 'cat', 'dog'])
      expect(result.excludeTags).toEqual([])
    })
  })

  describe('tag exclusion', () => {
    it('should parse single excluded tag', () => {
      const result = parseTagSearch('-dog')
      expect(result.includeTags).toEqual([])
      expect(result.excludeTags).toEqual(['dog'])
    })

    it('should parse mixed include and exclude', () => {
      const result = parseTagSearch('girl cat -dog')
      expect(result.includeTags).toEqual(['girl', 'cat'])
      expect(result.excludeTags).toEqual(['dog'])
    })

    it('should handle multiple excluded tags', () => {
      const result = parseTagSearch('-dog -cat -bird')
      expect(result.includeTags).toEqual([])
      expect(result.excludeTags).toEqual(['dog', 'cat', 'bird'])
    })

    it('should ignore empty exclusion (standalone hyphen)', () => {
      const result = parseTagSearch('girl - cat')
      // The standalone '-' creates an empty string after substring(1)
      expect(result.includeTags).toEqual(['girl', 'cat'])
    })
  })

  describe('OR logic', () => {
    it('should parse simple OR', () => {
      const result = parseTagSearch('girl or cat')
      expect(result.includeTags).toEqual([])
      expect(result.orGroups).toEqual([['girl', 'cat']])
    })

    it('should parse multiple OR groups', () => {
      const result = parseTagSearch('girl or cat boy or dog')
      expect(result.includeTags).toEqual([])
      expect(result.orGroups).toEqual([['girl', 'cat'], ['boy', 'dog']])
    })

    it('should handle chained OR (three tags)', () => {
      const result = parseTagSearch('girl or cat or dog')
      expect(result.includeTags).toEqual([])
      expect(result.orGroups).toEqual([['girl', 'cat', 'dog']])
    })

    it('should parse mixed AND and OR', () => {
      const result = parseTagSearch('anime girl or boy')
      expect(result.includeTags).toEqual(['anime'])
      expect(result.orGroups).toEqual([['girl', 'boy']])
    })

    it('should handle OR with exclusion', () => {
      const result = parseTagSearch('girl or cat -dog')
      expect(result.includeTags).toEqual([])
      expect(result.orGroups).toEqual([['girl', 'cat']])
      expect(result.excludeTags).toEqual(['dog'])
    })

    it('should handle case-insensitive OR', () => {
      const result = parseTagSearch('girl OR cat')
      expect(result.orGroups).toEqual([['girl', 'cat']])
    })

    it('should handle OR at start (edge case)', () => {
      const result = parseTagSearch('or cat')
      // 'or' at position 0, no previous tag, should not create OR group
      expect(result.includeTags).toEqual(['cat'])
      expect(result.orGroups).toEqual([])
    })

    it('should handle OR at end (edge case)', () => {
      const result = parseTagSearch('girl or')
      // 'or' at end, no next tag, should not create OR group
      expect(result.includeTags).toEqual(['girl'])
      expect(result.orGroups).toEqual([])
    })
  })

  describe('rating filters', () => {
    it('should parse single rating', () => {
      const result = parseTagSearch('rating:g')
      expect(result.ratings).toEqual(['g'])
    })

    it('should parse multiple ratings (comma-separated)', () => {
      const result = parseTagSearch('rating:g,s,q')
      expect(sorted(result.ratings)).toEqual(sorted(['g', 's', 'q']))
    })

    it('should parse multiple separate rating: tags', () => {
      const result = parseTagSearch('rating:g rating:s')
      expect(sorted(result.ratings)).toEqual(sorted(['g', 's']))
    })

    it('should handle full rating names', () => {
      const result = parseTagSearch('rating:general')
      expect(result.ratings).toEqual(['g'])
    })

    it('should parse all rating variations', () => {
      const inputs = [
        'rating:general',
        'rating:sensitive',
        'rating:questionable',
        'rating:explicit',
      ]
      const expected = ['g', 's', 'q', 'e']
      inputs.forEach((input, i) => {
        const result = parseTagSearch(input)
        expect(result.ratings).toEqual([expected[i]])
      })
    })

    it('should be case-insensitive', () => {
      const result = parseTagSearch('RATING:G rating:S')
      expect(sorted(result.ratings)).toEqual(sorted(['g', 's']))
    })

    it('should parse rating with tags', () => {
      const result = parseTagSearch('girl rating:s cat')
      expect(result.includeTags).toEqual(['girl', 'cat'])
      expect(result.ratings).toEqual(['s'])
    })

    it('should list a repeated rating once (the Set these arrays replaced)', () => {
      const result = parseTagSearch('rating:s rating:s,q rating:questionable')
      expect(sorted(result.ratings)).toEqual(sorted(['s', 'q']))
    })
  })

  describe('file type filters (is:)', () => {
    it('should parse single type', () => {
      const result = parseTagSearch('is:png')
      expect(result.fileTypes).toEqual(['image/png'])
    })

    it('should parse jpg and jpeg', () => {
      const result1 = parseTagSearch('is:jpg')
      const result2 = parseTagSearch('is:jpeg')
      expect(result1.fileTypes).toEqual(['image/jpeg'])
      expect(result2.fileTypes).toEqual(['image/jpeg'])
    })

    it('should parse all supported types', () => {
      const result = parseTagSearch('is:png is:jpg is:webp is:gif is:svg')
      expect(sorted(result.fileTypes)).toEqual(sorted([
        'image/png',
        'image/jpeg',
        'image/webp',
        'image/gif',
        'image/svg+xml',
      ]))
    })

    it('should be case-insensitive', () => {
      const result = parseTagSearch('IS:PNG is:JPG')
      expect(sorted(result.fileTypes)).toEqual(sorted(['image/png', 'image/jpeg']))
    })

    it('should parse is:unrated flag', () => {
      const result = parseTagSearch('is:unrated')
      expect(result.includeUnrated).toBe(true)
      expect(result.fileTypes).toEqual([])
    })

    it('should parse mixed is: tags', () => {
      const result = parseTagSearch('is:png is:unrated')
      expect(result.fileTypes).toEqual(['image/png'])
      expect(result.includeUnrated).toBe(true)
    })

    it('should list jpg and jpeg once, not twice (they map to one MIME type)', () => {
      const result = parseTagSearch('is:jpg is:jpeg')
      expect(result.fileTypes).toEqual(['image/jpeg'])
    })
  })

  describe('account filters', () => {
    it('should parse a single account', () => {
      const result = parseTagSearch('account:alice')
      expect(result.accounts).toEqual(['alice'])
      expect(result.excludeAccounts).toEqual([])
      expect(result.includeTags).toEqual([])
    })

    it('should parse a comma-separated list', () => {
      const result = parseTagSearch('account:alice,bob')
      expect(sorted(result.accounts)).toEqual(sorted(['alice', 'bob']))
    })

    it('should parse exclusions', () => {
      const result = parseTagSearch('-account:alice -account:bob,carol')
      expect(result.accounts).toEqual([])
      expect(sorted(result.excludeAccounts)).toEqual(sorted(['alice', 'bob', 'carol']))
    })

    it('should keep accounts and exclusions apart, alongside tags', () => {
      const result = parseTagSearch('girl account:alice -account:bob')
      expect(result.includeTags).toEqual(['girl'])
      expect(result.accounts).toEqual(['alice'])
      expect(result.excludeAccounts).toEqual(['bob'])
    })

    it('should list a repeated account once', () => {
      const result = parseTagSearch('account:alice account:alice,bob')
      expect(sorted(result.accounts)).toEqual(sorted(['alice', 'bob']))
    })
  })

  // Design D15: this branch used to ask `remainingQuery` whether it matched and
  // then read the captures off `query`. The two are the same string only
  // because tagcount is parsed FIRST, so no input can tell the shapes apart and
  // no case fails before the fix. These pin what the one-read restructure has
  // to preserve, and are what breaks if the parse steps are ever reordered.
  describe('collection filters', () => {
    it('should parse a single collection', () => {
      const result = parseTagSearch('collection:favorites')
      expect(result.collections).toEqual(['favorites'])
      expect(result.excludeCollections).toEqual([])
      expect(result.includeTags).toEqual([])
    })

    it('should lower-case the slug, matching what Rust computed', () => {
      const result = parseTagSearch('collection:Favorites')
      expect(result.collections).toEqual(['favorites'])
    })

    it('should parse a comma-separated list', () => {
      const result = parseTagSearch('collection:favorites,queue')
      expect(sorted(result.collections)).toEqual(sorted(['favorites', 'queue']))
    })

    it('should parse exclusions', () => {
      const result = parseTagSearch('-collection:favorites -collection:queue,to_upload')
      expect(result.collections).toEqual([])
      expect(sorted(result.excludeCollections)).toEqual(sorted(['favorites', 'queue', 'to_upload']))
    })

    it('should keep collections and exclusions apart, alongside tags', () => {
      const result = parseTagSearch('cat collection:favorites -collection:queue')
      expect(result.includeTags).toEqual(['cat'])
      expect(result.collections).toEqual(['favorites'])
      expect(result.excludeCollections).toEqual(['queue'])
    })

    it('should read a slug with a hyphen whole, as Rust computes it', () => {
      // `collections::slug` only lower-cases and turns whitespace into `_`, so
      // `To-upload` is the slug `to-upload`: a narrower charset here would
      // read the term the sidebar wrote as `collection:to`.
      const result = parseTagSearch('collection:to-upload -collection:re-run')
      expect(result.collections).toEqual(['to-upload'])
      expect(result.excludeCollections).toEqual(['re-run'])
      expect(result.includeTags).toEqual([])
    })

    it('should list a repeated collection once', () => {
      const result = parseTagSearch('collection:favorites collection:favorites,queue')
      expect(sorted(result.collections)).toEqual(sorted(['favorites', 'queue']))
    })
  })

  describe('collection:none / collection:any', () => {
    it('reads collection:none as noCollection, no slug', () => {
      const result = parseTagSearch('collection:none')
      expect(result.noCollection).toBe(true)
      expect(result.anyCollection).toBe(false)
      expect(result.collections).toEqual([])
    })

    it('reads collection:any as anyCollection', () => {
      const result = parseTagSearch('collection:any')
      expect(result.anyCollection).toBe(true)
      expect(result.noCollection).toBe(false)
      expect(result.collections).toEqual([])
    })

    it('reads -collection:none as anyCollection', () => {
      const result = parseTagSearch('-collection:none')
      expect(result.anyCollection).toBe(true)
      expect(result.noCollection).toBe(false)
      expect(result.excludeCollections).toEqual([])
    })

    it('reads -collection:any as noCollection', () => {
      const result = parseTagSearch('-collection:any')
      expect(result.noCollection).toBe(true)
      expect(result.anyCollection).toBe(false)
      expect(result.excludeCollections).toEqual([])
    })

    it('is case-insensitive', () => {
      const result = parseTagSearch('COLLECTION:None')
      expect(result.noCollection).toBe(true)
    })

    it('reads collection:none_left as a slug, not the keyword', () => {
      const result = parseTagSearch('collection:none_left')
      expect(result.collections).toEqual(['none_left'])
      expect(result.noCollection).toBe(false)
      expect(result.anyCollection).toBe(false)
    })

    it('drops none/any inside a comma list, without setting the flag', () => {
      const result = parseTagSearch('collection:cute,none')
      expect(result.collections).toEqual(['cute'])
      expect(result.noCollection).toBe(false)
      expect(result.anyCollection).toBe(false)
    })
  })

  describe('tag count filters', () => {
    const operators: [suffix: string, filter: TagCountFilter][] = [
      ['2', { operator: '=', value: 2 }],
      ['>5', { operator: '>', value: 5 }],
      ['<3', { operator: '<', value: 3 }],
      ['>=2', { operator: '>=', value: 2 }],
      ['<=10', { operator: '<=', value: 10 }],
      ['1..10', { operator: 'range', min: 1, max: 10 }],
      ['10..1', { operator: 'range', min: 1, max: 10 }],
      ['1,3,5', { operator: 'list', values: [1, 3, 5] }],
      ['0,2,4,6,8', { operator: 'list', values: [0, 2, 4, 6, 8] }],
    ]

    // `COUNT_METATAGS` itself, not a hand-copied list of names: a sixth row
    // added there without a matching test case still runs every operator
    // form through it here (design D6).
    it.each(COUNT_METATAGS)('%s: reads every operator form and strips it from the tag terms', (name, category) => {
      for (const [suffix, filter] of operators) {
        const query = `${name}:${suffix}`
        expect(parseTagSearch(query).tagCountTerms).toEqual([{ category, filter }])

        const amongTags = parseTagSearch(`girl ${query} cat`)
        expect(amongTags.tagCountTerms).toEqual([{ category, filter }])
        expect(amongTags.includeTags).toEqual(['girl', 'cat'])
      }
    })

    it('reads a count metatag case-insensitively', () => {
      const result = parseTagSearch('COPYTAGS:0')
      expect(result.tagCountTerms).toEqual([{ category: 'copyright', filter: { operator: '=', value: 0 } }])
    })

    it('combines different count metatags, in table order', () => {
      const result = parseTagSearch('copytags:0 chartags:>0')
      expect(result.tagCountTerms).toEqual([
        { category: 'character', filter: { operator: '>', value: 0 } },
        { category: 'copyright', filter: { operator: '=', value: 0 } },
      ])
      expect(result.includeTags).toEqual([])
    })

    it('keeps the first of a repeated count metatag', () => {
      const result = parseTagSearch('copytags:0 copytags:>0')
      expect(result.tagCountTerms).toEqual([
        { category: 'copyright', filter: { operator: '=', value: 0 } },
      ])
      expect(result.includeTags).toEqual([])
    })

    it('keeps the first across forms', () => {
      const result = parseTagSearch('tagcount:5 tagcount:1,3')
      expect(result.tagCountTerms).toEqual([{ category: null, filter: { operator: '=', value: 5 } }])
    })

    it('drops a leading minus, not as an exclusion', () => {
      const result = parseTagSearch('-copytags:0')
      expect(result.tagCountTerms).toEqual([{ category: 'copyright', filter: { operator: '=', value: 0 } }])
      expect(result.excludeTags).toEqual([])
    })

    it('takes the first tagcount and strips every one of them', () => {
      // The shape the defect would have shown up in: if the captures were read
      // off a string an earlier step had not stripped, the two occurrences
      // would disagree about which one won.
      const result = parseTagSearch('girl tagcount:>2 tagcount:5 cat')
      expect(result.tagCountTerms).toEqual([{ category: null, filter: { operator: '>', value: 2 } }])
      expect(result.includeTags).toEqual(['girl', 'cat'])
    })

    it('parses the same tagcount wherever it sits among the other metatags', () => {
      for (const query of [
        'rating:s tagcount:>2 cat',
        'tagcount:>2 rating:s cat',
        'cat rating:s is:png account:alice tagcount:>2',
        'tagcount:>2 cat rating:s is:png account:alice',
      ]) {
        const result = parseTagSearch(query)
        expect(result.tagCountTerms).toEqual([{ category: null, filter: { operator: '>', value: 2 } }])
        expect(result.includeTags).toEqual(['cat'])
      }
    })
  })

  describe('complex combinations', () => {
    it('should parse all filter types together', () => {
      const result = parseTagSearch('girl cat or boy -dog rating:s is:png tagcount:>2')
      expect(result.includeTags).toEqual(['girl'])
      expect(result.orGroups).toEqual([['cat', 'boy']])
      expect(result.excludeTags).toEqual(['dog'])
      expect(result.ratings).toEqual(['s'])
      expect(result.fileTypes).toEqual(['image/png'])
      expect(result.tagCountTerms).toEqual([{ category: null, filter: { operator: '>', value: 2 } }])
    })

    it('should handle multiple metatags of same type', () => {
      const result = parseTagSearch('rating:g,s is:png is:jpg tagcount:1..5')
      expect(sorted(result.ratings)).toEqual(sorted(['g', 's']))
      expect(sorted(result.fileTypes)).toEqual(sorted(['image/png', 'image/jpeg']))
      expect(result.tagCountTerms).toEqual([{ category: null, filter: { operator: 'range', min: 1, max: 5 } }])
    })

    it('should handle real-world query', () => {
      const result = parseTagSearch(
        'anime girl long_hair or short_hair -realistic rating:g,s is:png is:jpg tagcount:3..10',
      )
      expect(result.includeTags).toEqual(['anime', 'girl'])
      expect(result.orGroups).toEqual([['long_hair', 'short_hair']])
      expect(result.excludeTags).toEqual(['realistic'])
      expect(sorted(result.ratings)).toEqual(sorted(['g', 's']))
      expect(sorted(result.fileTypes)).toEqual(sorted(['image/png', 'image/jpeg']))
      expect(result.tagCountTerms).toEqual([{ category: null, filter: { operator: 'range', min: 3, max: 10 } }])
    })
  })

  // `lowercase-tags` design D1, D3: Rust stores every tag canonical (lower
  // case), so the parsed view of a query has to fold the same way or a typed
  // capital would ask for a tag that cannot exist. The metatags below are
  // unaffected — they were already parsed case-insensitively and read into a
  // fixed alphabet, never into free tag text.
  describe('tag case', () => {
    it('lower-cases include, exclude and or-group tags', () => {
      const result = parseTagSearch('Cat -Dog {A ~ b}')
      expect(result.includeTags).toEqual(['cat', '{a', '~', 'b}'])
      expect(result.excludeTags).toEqual(['dog'])
      expect(result.orGroups).toEqual([])
    })

    it('lower-cases both sides of an or group', () => {
      const result = parseTagSearch('Girl or Cat')
      expect(result.orGroups).toEqual([['girl', 'cat']])
    })

    it('leaves rating, is: and account: behaviour unchanged by capitals', () => {
      const result = parseTagSearch('Cat RATING:S IS:PNG ACCOUNT:Alice')
      expect(result.includeTags).toEqual(['cat'])
      expect(result.ratings).toEqual(['s'])
      expect(result.fileTypes).toEqual(['image/png'])
      // `account:` keeps the handle's own case (design D4, D6): only the tag
      // terms below it are folded.
      expect(result.accounts).toEqual(['Alice'])
    })

    // Design D1's argument for `to_lowercase` over an ASCII rule in Rust:
    // `toLowerCase()` here is already Unicode-aware, so a non-ASCII capital
    // folds the same way Rust folds it.
    it('lower-cases a non-ASCII capital', () => {
      const result = parseTagSearch('ÉTÉ')
      expect(result.includeTags).toEqual(['été'])
    })
  })

  describe('edge cases', () => {
    it('should handle only metatags (no regular tags)', () => {
      const result = parseTagSearch('rating:g is:png tagcount:5')
      expect(result.includeTags).toEqual([])
      expect(result.ratings).toEqual(['g'])
      expect(result.fileTypes).toEqual(['image/png'])
      expect(result.tagCountTerms).toEqual([{ category: null, filter: { operator: '=', value: 5 } }])
    })

    it('should handle invalid rating values', () => {
      const result = parseTagSearch('rating:x')
      // Invalid rating char, first char is 'x' which is not g/s/q/e
      // The regex should not match 'rating:x' at all
      expect(result.ratings).toEqual([])
    })

    it('should handle incomplete metatags', () => {
      const result1 = parseTagSearch('rating:')
      const result2 = parseTagSearch('tagcount:')
      const result3 = parseTagSearch('is:')
      // Incomplete metatags should not match regex
      expect(result1.ratings).toEqual([])
      expect(result2.tagCountTerms).toEqual([])
      expect(result3.fileTypes).toEqual([])
    })

    it('should handle consecutive OR operators', () => {
      const result = parseTagSearch('girl or or cat')
      // 'or or' - first 'or' creates group ['girl', 'or'], second 'or' creates group ['or', 'cat']
      // This is weird but tests current behavior
      expect(result.orGroups.length).toBeGreaterThan(0)
    })

    it('should preserve underscores in tags', () => {
      const result = parseTagSearch('long_hair short_hair')
      expect(result.includeTags).toEqual(['long_hair', 'short_hair'])
    })

    it('should preserve special characters in tags', () => {
      const result = parseTagSearch('girl_(qualifier) cat:pet')
      expect(result.includeTags).toEqual(['girl_(qualifier)', 'cat:pet'])
    })
  })
})
