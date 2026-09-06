import { describe, expect, it } from 'vitest'
import {
  applySuggestion,
  completeToken,
  confirmAction,
  currentToken,
  filterSuggestions,
  initialHighlight,
  isTokenIncomplete,
  moveHighlight,
  SUGGESTION_LIMIT,
  suggestionPrefix,
} from './tag-input'

describe('currentToken', () => {
  it('is the word the caret is at the end of', () => {
    expect(currentToken('cat', 3)).toEqual({ start: 0, end: 3, typed: 'cat' })
  })

  it('is the last word when the caret is at the end of a query', () => {
    expect(currentToken('dog cat', 7)).toEqual({ start: 4, end: 7, typed: 'cat' })
  })

  it('reaches past the caret to the end of the word', () => {
    expect(currentToken('cathedral dog', 4)).toEqual({ start: 0, end: 9, typed: 'cath' })
  })

  it('is empty after a trailing space', () => {
    expect(currentToken('cat ', 4)).toEqual({ start: 4, end: 4, typed: '' })
  })

  it('is empty at the start of a query', () => {
    expect(currentToken(' cat', 0)).toEqual({ start: 0, end: 0, typed: '' })
  })
})

describe('suggestionPrefix', () => {
  it('is the token being typed', () => {
    expect(suggestionPrefix('dog cat', 7)).toBe('cat')
  })

  it('drops the exclusion prefix', () => {
    expect(suggestionPrefix('-cathe', 6)).toBe('cathe')
  })

  it('is empty after a space, which lists the vocabulary', () => {
    expect(suggestionPrefix('cat ', 4)).toBe('')
  })

  it('is null inside a metatag', () => {
    expect(suggestionPrefix('rating:', 7)).toBeNull()
    expect(suggestionPrefix('is:un', 5)).toBeNull()
    expect(suggestionPrefix('tagcount:2', 10)).toBeNull()
    expect(suggestionPrefix('account:foo', 11)).toBeNull()
  })

  it('is null on the or operator and the o it starts as', () => {
    expect(suggestionPrefix('cat or', 6)).toBeNull()
    expect(suggestionPrefix('cat OR', 6)).toBeNull()
    expect(suggestionPrefix('cat o', 5)).toBeNull()
  })
})

describe('filterSuggestions', () => {
  it('drops tags the input already names, the one being typed included', () => {
    expect(filterSuggestions('cat cath', ['cat', 'cathedral', 'cath'])).toEqual(['cathedral'])
  })

  it('drops excluded tags and tags inside an or group', () => {
    expect(filterSuggestions('-dog fox or owl', ['dog', 'fox', 'owl', 'bat'])).toEqual(['bat'])
  })

  it('matches case-insensitively', () => {
    expect(filterSuggestions('Cat', ['cat', 'cathedral'])).toEqual(['cathedral'])
  })

  it('caps the list', () => {
    const many = Array.from({ length: 20 }, (_, i) => `tag${i}`)
    expect(filterSuggestions('', many)).toHaveLength(SUGGESTION_LIMIT)
  })
})

describe('initialHighlight', () => {
  it('takes the first row while something has been typed', () => {
    expect(initialHighlight('cat', 3)).toBe(0)
  })

  it('takes no row with an empty prefix', () => {
    expect(initialHighlight('', 3)).toBe(-1)
  })

  it('takes no row with an empty list', () => {
    expect(initialHighlight('cat', 0)).toBe(-1)
  })
})

describe('moveHighlight', () => {
  it('moves down to the last row and stops', () => {
    expect(moveHighlight(0, 1, 3)).toBe(1)
    expect(moveHighlight(2, 1, 3)).toBe(2)
  })

  it('moves up past the first row to nothing highlighted, and stops', () => {
    expect(moveHighlight(0, -1, 3)).toBe(-1)
    expect(moveHighlight(-1, -1, 3)).toBe(-1)
  })

  it('highlights nothing when there is nothing to highlight', () => {
    expect(moveHighlight(-1, 1, 0)).toBe(-1)
  })
})

describe('applySuggestion', () => {
  it('replaces the token being typed and finishes it with a space', () => {
    expect(applySuggestion('cat', 3, 'cathedral')).toEqual({ value: 'cathedral ', caret: 10 })
  })

  it('keeps an exclusion prefix', () => {
    expect(applySuggestion('-cathe', 6, 'cathedral')).toEqual({ value: '-cathedral ', caret: 11 })
  })

  it('steps over a space already following the token, rather than doubling it', () => {
    expect(applySuggestion('dog cat rating:s', 7, 'cathedral'))
      .toEqual({ value: 'dog cathedral rating:s', caret: 14 })
  })

  it('replaces the whole word the caret is inside', () => {
    expect(applySuggestion('cat dog', 3, 'cathedral'))
      .toEqual({ value: 'cathedral dog', caret: 10 })
  })
})

describe('isTokenIncomplete', () => {
  it('is true at the end of a word', () => {
    expect(isTokenIncomplete('cat', 3)).toBe(true)
  })

  it('is false after a trailing space', () => {
    expect(isTokenIncomplete('cat ', 4)).toBe(false)
  })

  it('is false in an empty input', () => {
    expect(isTokenIncomplete('', 0)).toBe(false)
  })
})

describe('completeToken', () => {
  it('finishes the token with a space', () => {
    expect(completeToken('cat', 3)).toEqual({ value: 'cat ', caret: 4 })
  })

  it('steps over a space that is already there', () => {
    expect(completeToken('cathedral dog', 9)).toEqual({ value: 'cathedral dog', caret: 10 })
  })

  it('splits a word the caret is inside', () => {
    expect(completeToken('catdog', 3)).toEqual({ value: 'cat dog', caret: 4 })
  })
})

describe('confirmAction', () => {
  it('accepts the highlighted suggestion first', () => {
    expect(confirmAction('cat', 3, true)).toBe('accept')
  })

  it('completes an unfinished token when nothing is highlighted', () => {
    expect(confirmAction('cat', 3, false)).toBe('complete')
  })

  it('submits when nothing is pending', () => {
    expect(confirmAction('cat ', 4, false)).toBe('submit')
    expect(confirmAction('', 0, false)).toBe('submit')
  })

  // Design D13, spec `tag-editing`: accept, then submit — accepting finishes the
  // token itself (task 7.5), so the confirm key needs only two presses here.
  it('runs accept then submit over two confirmations', () => {
    let text = { value: 'cat', caret: 3 }
    let highlighted = true

    expect(confirmAction(text.value, text.caret, highlighted)).toBe('accept')
    text = applySuggestion(text.value, text.caret, 'cathedral')
    // Accepting closes the list and finishes the token, so the next
    // confirmation has no highlight and nothing left pending.
    highlighted = false
    expect(text).toEqual({ value: 'cathedral ', caret: 10 })

    expect(confirmAction(text.value, text.caret, highlighted)).toBe('submit')
  })

  it('keeps a tag the library does not have and submits on the second confirmation', () => {
    const typed = { value: 'brandnew', caret: 8 }
    expect(confirmAction(typed.value, typed.caret, false)).toBe('complete')
    const finished = completeToken(typed.value, typed.caret)
    expect(finished).toEqual({ value: 'brandnew ', caret: 9 })
    expect(confirmAction(finished.value, finished.caret, false)).toBe('submit')
  })
})
