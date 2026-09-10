// The artist a page address suggests, ported from the legacy
// `extractArtistFromUrl` (`booru-upload` design D11). It is prefill and nothing
// else: it proposes a string the upload form shows and the user overwrites, so
// it stays in the webview beside the form rather than crossing to Rust.
//
// Each rule below may overwrite the one above it, which is the legacy order and
// is kept: an address matching two rules is answered by the later one.

/** What a rule proposes: either half may be absent when nothing matched. */
export interface ArtistCandidate {
  /** The artist tag to offer. Absent for a rule that only recognises the site. */
  artist?: string
  /** The address the matched rule considers the post's source. */
  source?: string
}

export function extractArtistFromUrl(url: string): ArtistCandidate {
  const result: ArtistCandidate = {}

  const pixivUser = url.match(/pixiv\.net\/(?:en\/)?users\/(\d+)/)
  const pixivArtwork = url.match(/pixiv\.net\/(?:en\/)?artworks\/(\d+)/)
  if (pixivUser) {
    result.artist = `pixiv_user_${pixivUser[1]}`
    result.source = url
  } else if (pixivArtwork) {
    result.source = url
  }

  // The three reserved paths are pages of the site, not of a person.
  const twitter = url.match(/(?:twitter|x)\.com\/([^/]+)/)
  if (twitter && !['i', 'home', 'search'].includes(twitter[1])) {
    result.artist = twitter[1]
    result.source = url
  }

  // `/` is excluded from the capture as well as `.`: on `[^.]+` — the legacy's
  // class — the capture runs back over `https://` to the start of the string
  // and the scheme lands in the tag (`https://someone_fanbox`). Keep the class
  // to one host label.
  const fanbox = url.match(/([^./]+)\.fanbox\.cc/)
  if (fanbox) {
    result.artist = `${fanbox[1]}_fanbox`
    result.source = url
  }

  const deviantart = url.match(/deviantart\.com\/([^/]+)/)
  if (deviantart) {
    result.artist = deviantart[1]
    result.source = url
  }

  // ArtStation puts the artist in a path segment that is not always there, so
  // the rule recognises the site and proposes no tag.
  const artstation = url.match(/artstation\.com\/(?:artwork\/|[^/]+$)/)
  if (artstation) {
    result.source = url
  }

  return result
}
