// Site adapters: extraction only. An adapter reads a page and names what it
// found; it never decides what the fields mean — no tags, no rating, no artist
// concept (CLAUDE.md: extraction lives in the extension, policy lives in the
// app; spec `site-adapters`).

import type { SiteAdapterRecord } from '@boorubox/shared'

import { pixiv } from './pixiv.js'
import type { AdapterFields, ExtractContext, PageContext, SiteAdapter } from './types.js'
import { x } from './x.js'

export type { AdapterFields, ExtractContext, PageContext, SiteAdapter } from './types.js'

/** The adapters that ship. */
export const ADAPTERS: SiteAdapter[] = [x, pixiv]

export function findAdapter(hostname: string, adapters: SiteAdapter[] = ADAPTERS) {
  return adapters.find((adapter) => adapter.hosts.some((host) => hostMatches(hostname, host)))
}

function hostMatches(hostname: string, host: string) {
  return hostname === host || hostname.endsWith(`.${host}`)
}

/**
 * What one page can say about itself: its adapter record, and what it is called.
 *
 * Every failure is the same answer — no record — and none of them stops the
 * capture (spec `site-adapters`: "A failing adapter never blocks a capture").
 * A site whose markup moved is indistinguishable here from a site nobody wrote
 * an adapter for, and both are harmless: the image still reaches the app.
 */
export function extractPageContext(
  context: ExtractContext,
  adapters: SiteAdapter[] = ADAPTERS,
): PageContext {
  const adapter = findAdapter(context.hostname, adapters)
  const record = adapter ? recordOf(adapter, context) : null
  return {
    record,
    // `document.title` read here is the same string `chrome.tabs` would report,
    // one moment fresher. What actually rescues a page is the adapter above it:
    // a site that never rewrites its title on a route has nothing fresher to
    // give, and only the post itself says what is on screen.
    pageTitle: titleOf(adapter, record) ?? context.document.title,
  }
}

function recordOf(adapter: SiteAdapter, context: ExtractContext): SiteAdapterRecord | null {
  try {
    // The site is kept even when every field turned out to be unreadable: it is
    // the one thing the match itself proved, and the app files the image under
    // it.
    return { site: adapter.site, fields: presentFields(adapter.extract(context)) }
  } catch {
    return null
  }
}

/**
 * Caught apart from the record: a title an adapter could not build is a page
 * that falls back to `document.title`, never a record that goes missing.
 */
function titleOf(
  adapter: SiteAdapter | undefined,
  record: SiteAdapterRecord | null,
): string | undefined {
  if (!adapter?.title || !record) {
    return undefined
  }
  try {
    return adapter.title(record.fields) || undefined
  } catch {
    return undefined
  }
}

/**
 * Drop what the page did not have. "The markup changed" and "the post had no
 * text" become the same shape, so nothing downstream has to tell an empty
 * string from a missing element (design D8).
 */
function presentFields(fields: AdapterFields): Record<string, string | string[]> {
  const present: Record<string, string | string[]> = {}
  for (const [name, value] of Object.entries(fields)) {
    if (typeof value === 'string' && value !== '') {
      present[name] = value
    } else if (Array.isArray(value) && value.length > 0) {
      present[name] = value
    }
  }
  return present
}
