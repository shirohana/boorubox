import { describe, expect, it } from 'vitest'
import {
  computeRatingCounts,
  filterImages,
  parseSearchQuery,
  sortImages,
  type FilterInputs,
} from './filters'
import { img } from './image-fixture'

const inputs = (o: Partial<FilterInputs> = {}): FilterInputs => ({
  view: o.view ?? 'all',
  urlSearch: o.urlSearch ?? '',
  tagSearch: o.tagSearch ?? '',
})

describe('parseSearchQuery', () => {
  it('parses a list tagcount and strips it from terms', () => {
    expect(parseSearchQuery('cat tagcount:1,3,5')).toEqual({
      terms: 'cat',
      tagCount: { operator: 'list', values: [1, 3, 5] },
    })
  })

  it('parses comparison and range operators', () => {
    expect(parseSearchQuery('tagcount:>2').tagCount).toEqual({ operator: '>', value: 2 })
    expect(parseSearchQuery('tagcount:<=4').tagCount).toEqual({ operator: '<=', value: 4 })
    expect(parseSearchQuery('tagcount:2..10').tagCount).toEqual({
      operator: 'range',
      min: 2,
      max: 10,
    })
    expect(parseSearchQuery('tagcount:3').tagCount).toEqual({ operator: '=', value: 3 })
  })

  it('returns null tagCount when absent', () => {
    expect(parseSearchQuery('just text').tagCount).toBeNull()
  })
})

describe('filterImages', () => {
  const all = [
    img({ id: 'a', deletedAt: null, tags: ['cat'], rating: 's' }),
    img({ id: 'b', deletedAt: null, tags: ['dog'], rating: 'e' }),
    img({ id: 'c', deletedAt: 5, tags: ['cat'], rating: 's' }),
  ]

  it('filters by view (all hides deleted, trash shows only deleted)', () => {
    expect(filterImages(all, inputs({ view: 'all' })).map((i) => i.id)).toEqual(['a', 'b'])
    expect(filterImages(all, inputs({ view: 'trash' })).map((i) => i.id)).toEqual(['c'])
  })

  it('filters by URL/title substring (case-insensitive)', () => {
    const imgs = [
      img({ id: 'x', pageTitle: 'Cute Cat' }),
      img({ id: 'y', pageUrl: 'https://dogs.example.com/1' }),
    ]
    expect(filterImages(imgs, inputs({ urlSearch: 'cat' })).map((i) => i.id)).toEqual(['x'])
    expect(filterImages(imgs, inputs({ urlSearch: 'dogs' })).map((i) => i.id)).toEqual(['y'])
  })

  it('applies include / exclude tags', () => {
    expect(filterImages(all, inputs({ tagSearch: 'cat' })).map((i) => i.id)).toEqual(['a'])
    expect(filterImages(all, inputs({ tagSearch: '-cat' })).map((i) => i.id)).toEqual(['b'])
  })

  it('applies the rating filter for the grid', () => {
    expect(filterImages(all, inputs({ tagSearch: 'rating:e' })).map((i) => i.id)).toEqual(['b'])
  })

  it('applies account: and -account: against the page URL', () => {
    const imgs = [
      img({ id: 'p', pageUrl: 'https://x.com/alice/status/1' }),
      img({ id: 'q', pageUrl: 'https://x.com/bob/status/1' }),
      img({ id: 'r', pageUrl: null }),
    ]
    expect(filterImages(imgs, inputs({ tagSearch: 'account:alice' })).map((i) => i.id))
      .toEqual(['p'])
    expect(filterImages(imgs, inputs({ tagSearch: '-account:alice' })).map((i) => i.id))
      .toEqual(['q', 'r'])
  })
})

describe('computeRatingCounts', () => {
  const all = [
    img({ id: 'a', tags: ['cat'], rating: 's' }),
    img({ id: 'b', tags: ['cat'], rating: 'e' }),
    img({ id: 'c', tags: ['cat'], rating: null }),
    img({ id: 'd', tags: ['dog'], rating: 's' }),
  ]

  it('counts ratings across the filtered set', () => {
    expect(computeRatingCounts(all, inputs())).toEqual({ g: 0, s: 2, q: 0, e: 1, unrated: 1 })
  })

  it('ignores the rating metatag so pills show counts for all ratings', () => {
    // tagSearch narrows to cat, but rating:s is intentionally NOT applied here
    expect(computeRatingCounts(all, inputs({ tagSearch: 'cat rating:s' }))).toEqual({
      g: 0, s: 1, q: 0, e: 1, unrated: 1,
    })
  })
})

describe('sortImages', () => {
  it('sorts capturedAt desc and asc in place', () => {
    const imgs = [
      img({ id: 'a', capturedAt: 1 }),
      img({ id: 'b', capturedAt: 3 }),
      img({ id: 'c', capturedAt: 2 }),
    ]
    sortImages(imgs, 'capturedAt-desc')
    expect(imgs.map((i) => i.id)).toEqual(['b', 'c', 'a'])
    sortImages(imgs, 'capturedAt-asc')
    expect(imgs.map((i) => i.id)).toEqual(['a', 'c', 'b'])
  })

  it('sorts by dimensions (area) and by updatedAt', () => {
    const imgs = [
      img({ id: 'small', width: 10, height: 10 }),
      img({ id: 'big', width: 100, height: 100 }),
    ]
    sortImages(imgs, 'dimensions-desc')
    expect(imgs.map((i) => i.id)).toEqual(['big', 'small'])

    const u = [
      img({ id: 'old', capturedAt: 1, updatedAt: 1 }),
      img({ id: 'new', capturedAt: 5, updatedAt: 2 }),
    ]
    sortImages(u, 'updatedAt-asc')
    expect(u.map((i) => i.id)).toEqual(['old', 'new'])
  })

  it('sorts by size', () => {
    const imgs = [img({ id: 'big', size: 900 }), img({ id: 'small', size: 100 })]
    sortImages(imgs, 'size-asc')
    expect(imgs.map((i) => i.id)).toEqual(['small', 'big'])
  })
})
