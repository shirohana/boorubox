// Opening a tag's Danbooru look-up (`tag-row-and-inspector-fixes` design D4): shared by the
// tag's context menu and the sidebar row's `?`, so the one FIXME below covers both callers
// instead of drifting into two.

import type { DanbooruLookup } from '$lib/domain/danbooru'
import { openExternal } from '$lib/api'

/**
 * FIXME(tag-row-and-inspector-fixes D4): the failure is dropped rather than shown — the URL
 * built by `danbooruLookup` is always `https:`, never one of `ExternalLink`'s two named
 * failures (a `file:` address, a malformed one), so there is nothing here to report beyond
 * "the browser refused". The right shape is a shared transient-notice surface neither the
 * menu (already closed by the time this resolves) nor the sidebar row (no line of its own to
 * show it in) has yet.
 */
export function openDanbooruLookup(lookup: DanbooruLookup): void {
  void openExternal(lookup.url)
}
