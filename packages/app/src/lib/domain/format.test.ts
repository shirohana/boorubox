import { describe, expect, it } from 'vitest'
import { formatBytes, formatTimestamp, ratingLabel } from './format'

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
