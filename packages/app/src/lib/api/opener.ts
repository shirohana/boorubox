// Opening an address outside this app. An `<a href>` to a booru would navigate
// the webview itself off the library screen, so every outward link goes to the
// operating system's browser through the opener plugin (`opener:default` in
// `src-tauri/capabilities/default.json`).

import { openUrl } from '@tauri-apps/plugin-opener'
import { errorText } from './errors'

/**
 * The reason the address could not be handed to the browser, or `null` when it
 * was. It resolves either way: the caller shows the reason where the link is,
 * because a link that does nothing at all is indistinguishable from a broken
 * address.
 */
export async function openExternal(url: string): Promise<string | null> {
  try {
    await openUrl(url)
    return null
  } catch (cause) {
    return errorText(cause)
  }
}
