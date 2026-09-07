// Pixiv. Selectors verified 2026-09-06 against two saved artwork pages;
// `__fixtures__/pixiv-artwork.html` is a trimmed copy of one, and its header
// carries the date (design D10).
//
// Every `class` on these pages is a styled-components hash that churns on any
// deploy, so nothing here selects on one. The `data-ga4-*` / `gtm-*` attributes
// and the `main` / `h1` / `h2` landmarks are the only durable hooks. The old
// `<meta id="meta-preload-data">` is gone from pixiv-web-next, and its
// `__NEXT_DATA__` still describes whatever page the tab loaded first — reading
// that gives the wrong artwork after any click-through.

import type { ExtractContext, SiteAdapter } from './types.js'

export const pixiv: SiteAdapter = {
  site: 'pixiv',
  hosts: ['pixiv.net'],

  extract({ document, imageUrl, pageUrl }: ExtractContext) {
    return {
      artist: readArtist(document),
      workId: readWorkId(document, pageUrl),
      title: readTitle(document),
      originalUrl: originalIllustUrl(document, imageUrl),
    }
  },
}

/** The address is the one part of an artwork page that cannot be restyled. */
function readWorkId(document: Document, pageUrl: string): string | undefined {
  return /\/artworks\/(\d+)/.exec(pageUrl)?.[1]
    ?? document.querySelector('[data-ga4-entity-id^="illust/"]')
      ?.getAttribute('data-ga4-entity-id')?.slice('illust/'.length)
      ?? /\/artworks\/(\d+)/.exec(canonical(document) ?? '')?.[1]
}

/**
 * `twitter:title` is the bare title; `og:title` and `<title>` are the localised
 * `#tag title - <artist>的插畫 - pixiv` template, which cannot be parsed without
 * knowing the viewer's language.
 */
function readTitle(document: Document): string | undefined {
  return document.querySelector('main h1')?.textContent?.trim()
    ?? document.querySelector('meta[property="twitter:title"]')?.getAttribute('content')
    ?? undefined
}

/**
 * Scoped to `main` on purpose: the same artist block is rendered again in the
 * recommendations below the fold, and the *signed-in* user's own avatar carries
 * the same `title` attribute up in the header. An unscoped selector picks
 * whichever comes first, which is the reader, not the artist.
 */
function readArtist(document: Document): string | undefined {
  return document.querySelector('main h2 a[href^="/users/"] div[title]')
    ?.getAttribute('title')
    ?? document.querySelector('main a[href^="/users/"] img[src*="/user-profile/"]')
      ?.getAttribute('alt')
      ?? undefined
}

/**
 * The original of the captured page of the work.
 *
 * Right-clicking the full-size view hands over an `img-original` address
 * already, page index and file extension included, and nothing beats that.
 * Anywhere else the captured address is a master or a thumbnail, which names
 * the page but not the extension — masters are always `.jpg` while originals
 * may be `.png` — so the extension has to come from the page's own full-size
 * link, and only the index is carried over from what was clicked.
 */
function originalIllustUrl(document: Document, imageUrl: string): string | undefined {
  if (imageUrl.includes('/img-original/')) {
    return imageUrl
  }
  const fullSize = document.querySelector('a.gtm-expand-full-size-illust')?.getAttribute('href')
  if (!fullSize) {
    return undefined
  }
  const page = /_p(\d+)/.exec(imageUrl)?.[1]
  return page === undefined ? fullSize : fullSize.replace(/_p\d+(\.\w+)$/, `_p${page}$1`)
}

function canonical(document: Document): string | undefined {
  return document.querySelector('link[rel="canonical"]')?.getAttribute('href') ?? undefined
}
