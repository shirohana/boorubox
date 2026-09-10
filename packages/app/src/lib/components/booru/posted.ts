// What "posted to <site>" is, for both places that show it: the Inspector's
// label and the grid tile's mark. `PostRef.site` is a stable slug, not a
// reference (design D2), so the join to the configured site is done here — and
// a record whose site is gone still identifies itself, with no link.

import type { BooruSite, PostRef } from '@boorubox/shared'

export interface PostedEntry {
  /** `PostRef.site`: the key the record was written with, and its identity. */
  key: string
  /** The configured site's name, or the key itself once the site is removed. */
  name: string
  remoteId: string
  postedAt: number
  /** `<baseUrl>/posts/<remoteId>`, or `null` for a site no longer configured. */
  url: string | null
}

/**
 * An address under a site's root. Every link this feature builds goes through
 * here: a base address the user typed may or may not end in a separator, and
 * two builders would eventually disagree about that.
 */
export function siteUrl(baseUrl: string, path: string): string {
  return `${baseUrl.replace(/\/+$/, '')}/${path}`
}

/** The post's address, computed and never stored (design D2). */
export function postUrl(baseUrl: string, remoteId: string): string {
  return siteUrl(baseUrl, `posts/${remoteId}`)
}

/** How a post is named wherever it is shown: the label, the tile, the dialog. */
export function postedName(name: string, remoteId: string): string {
  return `Posted to ${name} #${remoteId}`
}

function siteName(post: PostRef, sites: BooruSite[]): string {
  return sites.find((candidate) => candidate.id === post.site)?.name ?? post.site
}

export function postedEntries(posts: PostRef[], sites: BooruSite[]): PostedEntry[] {
  return posts.map((post) => {
    const site = sites.find((candidate) => candidate.id === post.site)
    return {
      key: post.site,
      name: site?.name ?? post.site,
      remoteId: post.remoteId,
      postedAt: post.postedAt,
      url: site ? postUrl(site.baseUrl, post.remoteId) : null,
    }
  })
}

/**
 * What the grid tile's mark says, one entry per record. Names only: the tile
 * links nothing, so it must not compute addresses it throws away — one join
 * per tile per redraw, across the whole grid.
 */
export function postedNames(posts: PostRef[], sites: BooruSite[]): string[] {
  return posts.map((post) => postedName(siteName(post, sites), post.remoteId))
}

/**
 * The sites an image may still be uploaded to (design D12): the presence of a
 * `posts` row is what withholds the action, so this is the whole of that rule.
 */
export function unpostedSites(posts: PostRef[], sites: BooruSite[]): BooruSite[] {
  const already = new Set(posts.map((post) => post.site))
  return sites.filter((site) => !already.has(site.id))
}
