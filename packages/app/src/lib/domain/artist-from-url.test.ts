import { expect, it } from 'vitest'
import { extractArtistFromUrl } from './artist-from-url'

it('reads a pixiv user id as an artist tag', () => {
  expect(extractArtistFromUrl('https://pixiv.net/en/users/12345')).toEqual({
    artist: 'pixiv_user_12345',
    source: 'https://pixiv.net/en/users/12345',
  })
})

it('recognises a pixiv artwork without naming an artist', () => {
  expect(extractArtistFromUrl('https://www.pixiv.net/artworks/999')).toEqual({
    source: 'https://www.pixiv.net/artworks/999',
  })
})

it('reads an X or Twitter handle', () => {
  expect(extractArtistFromUrl('https://x.com/alice/status/1').artist).toBe('alice')
  expect(extractArtistFromUrl('https://twitter.com/bob').artist).toBe('bob')
})

it('leaves the three reserved X paths without an artist', () => {
  for (const reserved of ['i', 'home', 'search']) {
    expect(extractArtistFromUrl(`https://x.com/${reserved}/whatever`).artist).toBeUndefined()
  }
})

it('reads a fanbox subdomain as the artist, with no scheme in the tag', () => {
  // The legacy's own test documented `https://someone_fanbox` here as a known
  // quirk; the capture is a host label now, so the scheme cannot leak in.
  expect(extractArtistFromUrl('https://someone.fanbox.cc/posts/1').artist).toBe('someone_fanbox')
  expect(extractArtistFromUrl('someone.fanbox.cc').artist).toBe('someone_fanbox')
})

it('reads a DeviantArt handle', () => {
  expect(extractArtistFromUrl('https://deviantart.com/carol/art/x').artist).toBe('carol')
})

it('recognises an ArtStation address without naming an artist', () => {
  expect(extractArtistFromUrl('https://artstation.com/artwork/abc')).toEqual({
    source: 'https://artstation.com/artwork/abc',
  })
  expect(extractArtistFromUrl('https://artstation.com/dave')).toEqual({
    source: 'https://artstation.com/dave',
  })
})

it('proposes nothing for an address no rule knows', () => {
  expect(extractArtistFromUrl('https://example.com/whatever')).toEqual({})
})
