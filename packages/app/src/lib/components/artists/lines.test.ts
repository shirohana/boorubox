import { describe, expect, it } from 'vitest'
import { lines } from './lines'

describe('lines', () => {
  it('drops blank lines', () => {
    expect(lines('https://x.com/a\n\n\nhttps://x.com/b\n')).toEqual([
      'https://x.com/a',
      'https://x.com/b',
    ])
  })

  it('splits on a CRLF line ending', () => {
    expect(lines('https://x.com/a\r\nhttps://x.com/b')).toEqual([
      'https://x.com/a',
      'https://x.com/b',
    ])
  })

  it('trims surrounding whitespace from each line', () => {
    expect(lines('  https://x.com/a  \n\thttps://x.com/b\t')).toEqual([
      'https://x.com/a',
      'https://x.com/b',
    ])
  })
})
