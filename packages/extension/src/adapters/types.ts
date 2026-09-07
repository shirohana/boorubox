// The adapter contract, apart from the registry so an adapter and the list it
// registers into do not import each other.

import type { SiteAdapterRecord } from '@boorubox/shared'

/** What an adapter read off the page. Absent fields are left out, not blanked. */
export type AdapterFields = Record<string, string | string[] | null | undefined>

export interface ExtractContext {
  document: Document
  /** The image the user asked to capture, as the context menu reported it. */
  imageUrl: string
  pageUrl: string
  hostname: string
}

export interface SiteAdapter {
  /** Goes into the record as `site`, and into the library as `source_ref`. */
  site: string
  /** Matched against the hostname and any subdomain of it. */
  hosts: string[]
  extract(context: ExtractContext): AdapterFields
  /**
   * What this page should be called, when the site's own `<title>` cannot be
   * trusted. Built from the fields `extract` already read, so the page is walked
   * once and the title can never name a different post than the record does.
   *
   * Still extraction, not policy: it answers "what is this page called", which
   * only the page can say. Nothing here interprets the fields — no tags, no
   * artist concept.
   */
  title?(fields: AdapterFields): string | undefined
}

/** Everything a tab can answer about the page one capture came from. */
export interface PageContext {
  /** The adapter record, or `null` when no adapter recognised the page. */
  record: SiteAdapterRecord | null
  /**
   * The page's title. An adapter's, when it has one; otherwise the live
   * `document.title` — which is what the tab reports too, so it helps only
   * against a lagging tab record, never against a site that leaves its own
   * title on the previous screen.
   */
  pageTitle: string
}
