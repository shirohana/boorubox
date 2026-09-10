// One wrapper per booru-upload Tauri command (design D12, the same rule
// `commands.ts` follows). Components import from here and never call
// `invoke` themselves.

import type {
  BooruConnectionTest,
  BooruSite,
  BooruUploadForm,
  BooruUploadOutcome,
} from '@boorubox/shared'
import { invoke } from '@tauri-apps/api/core'

/** The library's configured booru sites, ordered by name (design D1). */
export function booruSiteList(): Promise<BooruSite[]> {
  return invoke('booru_site_list')
}

/**
 * Creates a site when `id` is `null`, or edits the one it names — `id` itself
 * never changes on an edit, even one that changes `baseUrl`'s host (design
 * D2). `apiKey` left `null` leaves whatever credential the site already has
 * untouched: the form field it backs is write-only and starts blank on every
 * edit.
 *
 * The database write always happens before the credential is touched
 * (design D7): if the credential store then refuses `apiKey`, this call
 * rejects with that reason even though the site's other settings are already
 * saved — call {@link booruSiteList} again to see them.
 */
export function booruSiteSave(
  id: string | null,
  name: string,
  baseUrl: string,
  username: string,
  apiKey: string | null,
): Promise<BooruSite> {
  return invoke('booru_site_save', { id, name, baseUrl, username, apiKey })
}

/**
 * Removes a site and its stored credential; posts already recorded against
 * it are kept (`booru-sites` design D2).
 */
export function booruSiteDelete(id: string): Promise<void> {
  return invoke('booru_site_delete', { id })
}

/**
 * Contacts the site with its stored credential and reports what happened,
 * without changing anything on the booru (design D8). Rejects outright,
 * before any request, if the credential itself cannot be read (`booru-sites`
 * design D7).
 */
export function booruSiteTest(id: string): Promise<BooruConnectionTest> {
  return invoke('booru_site_test', { id })
}

/**
 * Runs the four-step upload sequence for `imageId` against `siteId` and
 * records the post only on success (design D5, D6). Never rejects for a
 * failure of the sequence itself — `BooruUploadOutcome.outcome === 'failed'`
 * names the step and the booru's own message; the promise only rejects for
 * what `AppError` is for: no library open, the image or site not found, or a
 * missing/refused credential, which fails before any request is sent
 * (`booru-sites` design D7).
 */
export function booruUpload(
  imageId: string,
  siteId: string,
  form: BooruUploadForm,
): Promise<BooruUploadOutcome> {
  return invoke('booru_upload', { imageId, siteId, form })
}
