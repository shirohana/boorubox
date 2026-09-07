// X (Twitter). Selectors verified 2026-09-06 against three saved photo-viewer
// pages; `__fixtures__/x-post.html` is a trimmed copy of one of them, and its
// header carries the date. When a field stops coming through, that date is what
// says how old the markup we last knew is (design D10).

import type { AdapterFields, ExtractContext, SiteAdapter } from './types.js'

/** Past this the title stops naming the post and starts being the post. */
const TITLE_TEXT_LIMIT = 100

export const x: SiteAdapter = {
  site: 'x',
  hosts: ['x.com', 'twitter.com'],

  extract({ document, imageUrl }: ExtractContext) {
    const post = focalPost(document, imageUrl)
    const permalink = readPermalink(post)
    return {
      handle: permalink?.handle,
      displayName: post ? readDisplayName(post) : undefined,
      postUrl: permalink?.url,
      postText: post ? readPostText(post) : undefined,
      originalUrl: originalMediaUrl(imageUrl),
    }
  },

  /**
   * X's own `<title>` is unusable here, and not only because it is localised:
   * X does not rewrite it when a photo is opened straight off a timeline, so the
   * page is still called "Home" while a post is on screen. Reading it live does
   * not help — there is nothing fresher to read. The post itself is in the DOM,
   * so the title is built from that, for every X capture and not only the stale
   * ones, or two captures of one post would be named differently.
   */
  title(fields: AdapterFields) {
    const author = readAuthor(text(fields.displayName), text(fields.handle))
    if (!author) {
      return undefined
    }
    const said = text(fields.postText)
    return said ? `${author} on X: ${firstLine(said)}` : `${author} on X`
  },
}

function text(value: AdapterFields[string]): string | undefined {
  return typeof value === 'string' && value !== '' ? value : undefined
}

/** X's own shape, minus what the markup did not give up. */
function readAuthor(displayName?: string, handle?: string): string | undefined {
  if (displayName && handle) {
    return `${displayName} (@${handle})`
  }
  return displayName ?? (handle && `@${handle}`) ?? undefined
}

/**
 * One line, short enough to read in a grid cell. The post's whole text is in
 * `postText`, so nothing is lost by cutting it here.
 */
function firstLine(said: string): string {
  const line = said.split('\n')[0]!.trim()
  return line.length > TITLE_TEXT_LIMIT ? `${line.slice(0, TITLE_TEXT_LIMIT).trimEnd()}…` : line
}

/**
 * The post the captured image belongs to.
 *
 * On a `/photo/N` page the full-size image sits in the modal overlay, outside
 * every `<article>`, and the page below it holds the post plus twenty of its
 * replies — so "the first article" is only right once the search is scoped to
 * that overlay. On a timeline the image is inside its own post's article, which
 * is the only way to tell one post from the nineteen around it.
 */
function focalPost(document: Document, imageUrl: string): Element | null {
  const mediaId = /\/media\/([A-Za-z0-9_-]+)/.exec(imageUrl)?.[1]
  const image = mediaId ? document.querySelector(`img[src*="/media/${mediaId}"]`) : null
  return image?.closest('article[data-testid="tweet"]')
    ?? image?.closest('[aria-modal="true"]')?.querySelector('article[data-testid="tweet"]')
    ?? document.querySelector('article[data-testid="tweet"]')
}

/**
 * The handle and the post id come from one element, the anchor around the
 * timestamp: it is the only place that carries both, so they cannot disagree.
 * There is no `<link rel=canonical>` and no `og:` meta on these pages, and
 * `<title>` is localised — neither is usable.
 */
function readPermalink(post: Element | null) {
  const href = post?.querySelector('a[role="link"]:has(time)')?.getAttribute('href')
  const match = href ? /^\/([^/]+)\/status\/(\d+)/.exec(href) : null
  if (!match) {
    return null
  }
  return { handle: match[1]!, url: `https://x.com/${match[1]}/status/${match[2]}` }
}

/**
 * The post's own words, minus any trailing `t.co` link — it addresses the media
 * that is being captured anyway, not something the post said.
 */
function readPostText(post: Element): string | undefined {
  const said = post.querySelector('[data-testid="tweetText"]')
  if (!said) {
    return undefined
  }
  const copy = said.cloneNode(true) as Element
  for (const link of copy.querySelectorAll('a[href^="https://t.co/"]')) {
    link.remove()
  }
  return readable(copy)
}

/**
 * The author's chosen name, as distinct from the handle. It is the first text
 * block inside the anchor to their profile; the verified badge is the block
 * after it, and the handle is a separate anchor.
 */
function readDisplayName(post: Element): string | undefined {
  const name = post.querySelector('[data-testid="User-Name"] a[role="link"] div[dir="ltr"]')
  return name ? readable(name) : undefined
}

/** X renders emoji as `<img alt>`, so the alt text is put back before reading. */
function readable(element: Element): string | undefined {
  const copy = element.cloneNode(true) as Element
  for (const emoji of copy.querySelectorAll('img[alt]')) {
    emoji.replaceWith(emoji.ownerDocument.createTextNode(emoji.getAttribute('alt') ?? ''))
  }
  return copy.textContent?.trim()
}

/**
 * Read off the image address rather than the page: `name` is the size X served,
 * and `orig` is the one it will serve. The DOM only ever holds the size that is
 * on screen.
 */
function originalMediaUrl(imageUrl: string): string | undefined {
  try {
    const url = new URL(imageUrl)
    if (!url.pathname.startsWith('/media/')) {
      return undefined
    }
    url.searchParams.set('name', 'orig')
    return url.href
  } catch {
    return undefined
  }
}
