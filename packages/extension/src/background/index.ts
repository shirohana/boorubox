// Service worker: nothing but wiring. Every decision this file reaches for
// lives in `deliver.ts`, `../delivery/state.ts` or `../history/store.ts`, so
// that what the extension does can be tested without a browser.

import { captureAndDeliver, retryAllFailed, retryCapture } from './deliver.js'
import { MENU_ID, registerContextMenu } from './menu.js'
import { refreshBadge, sweepInterrupted } from '../history/store.js'
import { RETRY_ALL, RETRY_CAPTURE, type PopupMessage } from '../messages.js'

chrome.runtime.onInstalled.addListener(() => {
  registerContextMenu()
})

// The worker is killed and restarted at will, so this runs on every start, not
// on install: an entry left `pending` belonged to a worker that is gone
// (design D4).
void sweepInterrupted().then(refreshBadge)

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId !== MENU_ID || !info.srcUrl || !tab?.id) {
    return
  }
  void captureAndDeliver({
    tabId: tab.id,
    imageUrl: info.srcUrl,
    pageUrl: info.pageUrl || tab.url || '',
    pageTitle: tab.title || '',
  })
})

chrome.runtime.onMessage.addListener((message: PopupMessage) => {
  if (message?.type === RETRY_CAPTURE) {
    void retryCapture(message.id)
  }
  if (message?.type === RETRY_ALL) {
    void retryAllFailed()
  }
  return false
})

// A shortcut to where the Retry is. The browser refuses `openPopup` outside a
// user gesture in some builds, and that is fine: the badge and the toolbar icon
// are the reliable path (design D15).
chrome.notifications.onClicked.addListener((id) => {
  void chrome.notifications.clear(id)
  void chrome.action.openPopup().catch(() => {})
})
