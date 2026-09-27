import { redirect } from '@sveltejs/kit'
import { resolve } from '$app/paths'
import { lastSettingsPage } from '$lib/components/settings/settings-pages'

/**
 * `/settings` itself addresses nothing (`settings-pages` design D2): it lands on whichever
 * page was last shown on this machine, or General the first time. A
 * universal load runs before anything renders (`ssr = false`), so nothing
 * flashes, and SvelteKit follows the redirect as the navigation's
 * destination — history holds the settings page's own address, not this one,
 * so Back never bounces forward again.
 */
export function load() {
  redirect(307, resolve(lastSettingsPage().path))
}
