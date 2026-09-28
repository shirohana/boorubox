import { describe, expect, it } from 'vitest'
import { formatBytes, formatRelative, formatSizeLine, formatTimestamp, ratingLabel } from './format'

describe('formatBytes', () => {
  it('leaves byte counts whole', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(1)).toBe('1 B')
    expect(formatBytes(999)).toBe('999 B')
  })

  it('steps up a unit at a thousand', () => {
    expect(formatBytes(1000)).toBe('1.0 kB')
    expect(formatBytes(1_500_000)).toBe('1.5 MB')
    expect(formatBytes(2_400_000_000)).toBe('2.4 GB')
  })

  it('drops the decimal once three digits are already showing', () => {
    expect(formatBytes(120_000)).toBe('120 kB')
  })

  it('stops at the largest unit it knows rather than inventing one', () => {
    expect(formatBytes(5e15)).toBe('5000 TB')
  })

  it('says nothing rather than something wrong for a size that is not one', () => {
    expect(formatBytes(-1)).toBe('—')
    expect(formatBytes(Number.NaN)).toBe('—')
  })
})

describe('formatTimestamp', () => {
  it('renders an epoch-millisecond column as a local date and time', () => {
    expect(formatTimestamp(0)).toBe(new Date(0).toLocaleString())
  })

  it('says nothing rather than "Invalid Date"', () => {
    expect(formatTimestamp(Number.NaN)).toBe('—')
  })
})

describe('ratingLabel', () => {
  it('spells out every stored letter', () => {
    expect(ratingLabel('g')).toBe('general')
    expect(ratingLabel('s')).toBe('sensitive')
    expect(ratingLabel('q')).toBe('questionable')
    expect(ratingLabel('e')).toBe('explicit')
  })

  it('calls no rating "unrated", the state `is:unrated` searches for', () => {
    expect(ratingLabel(null)).toBe('unrated')
  })
})

describe('formatSizeLine', () => {
  it('reads bytes, extension and pixel dimensions as one line', () => {
    expect(
      formatSizeLine({ size: 1_300_000, file: 'images/a1/b2/abc123.jpg', width: 1200, height: 2200 }),
    ).toBe('1.3 MB .jpg (1200×2200)')
  })

  it('omits the extension, and its leading space, for a file with none', () => {
    expect(formatSizeLine({ size: 1_300_000, file: 'images/a1/b2/abc123', width: 1200, height: 2200 })).toBe(
      '1.3 MB (1200×2200)',
    )
  })

  it('lower-cases an upper-case extension', () => {
    expect(
      formatSizeLine({ size: 1_300_000, file: 'images/a1/b2/abc123.JPG', width: 1200, height: 2200 }),
    ).toBe('1.3 MB .jpg (1200×2200)')
  })
})

describe('formatRelative', () => {
  const now = new Date('2026-09-28T12:00:00Z').getTime()
  const DAY = 24 * 60 * 60 * 1000

  it('reads under a minute as "just now"', () => {
    expect(formatRelative(now - 30_000, now)).toBe('just now')
  })

  it('reads one day ago as "yesterday"', () => {
    expect(formatRelative(now - DAY, now)).toBe('yesterday')
  })

  it('reads three days ago', () => {
    expect(formatRelative(now - 3 * DAY, now)).toBe('3 days ago')
  })

  it('reads two years ago', () => {
    expect(formatRelative(now - 2 * 365 * DAY, now)).toBe('2 years ago')
  })

  it('says nothing rather than something wrong for a time that is not one', () => {
    expect(formatRelative(Number.NaN, now)).toBe('—')
  })
})
