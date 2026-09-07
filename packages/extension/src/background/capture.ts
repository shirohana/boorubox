// Getting the bytes of a clicked image. Ported from the legacy extension's
// `src/background/index.ts` menu handler (requirements §4: canvas in the page
// first, background fetch with a `Referer` rule as the fallback).

import { dataUrlToBlob } from '../data-url.js'
import { CAPTURE_IMAGE, type CaptureResult } from '../messages.js'

/**
 * Dynamic-rule ids come from a counter, not from the clock (design D13,
 * departure 2). The legacy `Math.floor(Date.now() / 1000)` hands two captures
 * in the same second the same id, and then the first `finally` removes the rule
 * the second is still using — one of them fetches with no `Referer` and can 403.
 */
let nextRuleId = 1

/**
 * The clicked image's bytes: from the page if it can produce them, otherwise
 * fetched with the page as referrer. Raises only when neither route works.
 */
export async function captureImageBytes(
  tabId: number,
  imageUrl: string,
  pageUrl: string,
): Promise<Blob> {
  try {
    const response: CaptureResult | undefined = await chrome.tabs.sendMessage(tabId, {
      type: CAPTURE_IMAGE,
      imageUrl,
    })
    if (!response || 'error' in response) {
      throw new Error(response?.error ?? 'the page did not answer')
    }
    return dataUrlToBlob(response.dataUrl)
  } catch {
    return fetchWithReferer(imageUrl, pageUrl)
  }
}

/**
 * Hotlink protection reads `Referer`, and an extension's own fetch does not
 * send the page's. The rule that sets it is added for this one fetch and taken
 * away in a `finally`: left behind it would rewrite the header on every
 * request to that host, for as long as the profile lives.
 */
async function fetchWithReferer(imageUrl: string, pageUrl: string): Promise<Blob> {
  const ruleId = nextRuleId++
  try {
    const imageHost = new URL(imageUrl).host
    await chrome.declarativeNetRequest.updateDynamicRules({
      addRules: [
        {
          id: ruleId,
          priority: 1,
          action: {
            type: chrome.declarativeNetRequest.RuleActionType.MODIFY_HEADERS,
            requestHeaders: [
              {
                header: 'Referer',
                operation: chrome.declarativeNetRequest.HeaderOperation.SET,
                value: pageUrl,
              },
            ],
          },
          condition: {
            urlFilter: imageHost,
            resourceTypes: [chrome.declarativeNetRequest.ResourceType.XMLHTTPREQUEST],
          },
        },
      ],
      // The counter restarts with the worker, so a rule the previous worker was
      // killed before removing can still hold this id; adding onto it is an
      // error, and Chrome applies the removals first.
      removeRuleIds: [ruleId],
    })

    const response = await fetch(imageUrl)
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }
    return await response.blob()
  } finally {
    await chrome.declarativeNetRequest
      .updateDynamicRules({ addRules: [], removeRuleIds: [ruleId] })
      .catch(() => {})
  }
}
