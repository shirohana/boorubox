// Every notification this extension raises is titled `BooruBox` (design D12).
// While the legacy extension is still installed alongside it, a failure
// notification is one of only two places the two are confusable — the other is
// the context menu — and the legacy titles read "Failed to Save Image".

/** The icon a notification carries; `iconUrl` has no default in MV3. */
const ICON = 'icons/icon-48.png'

export const NOTIFICATION_TITLE = 'BooruBox'

export function notify(message: string, id?: string) {
  const options = {
    type: 'basic' as const,
    iconUrl: chrome.runtime.getURL(ICON),
    title: NOTIFICATION_TITLE,
    message,
  }
  return id ? chrome.notifications.create(id, options) : chrome.notifications.create(options)
}
