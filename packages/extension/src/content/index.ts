// Content script: the two things only the page can answer — the bytes of an
// image it has already loaded, and whatever a site adapter can read off it.
// Extraction only; what the fields mean is the app's business.

import { extractPageContext } from '../adapters/index.js'
import { captureImage } from './capture.js'
import { CAPTURE_IMAGE, EXTRACT_CONTEXT, type ContentMessage } from '../messages.js'

chrome.runtime.onMessage.addListener((message: ContentMessage, _sender, sendResponse) => {
  if (message?.type === CAPTURE_IMAGE) {
    captureImage(message.imageUrl).then(sendResponse)
    return true
  }

  if (message?.type === EXTRACT_CONTEXT) {
    sendResponse(extractPageContext({
      document,
      hostname: window.location.hostname,
      imageUrl: message.imageUrl,
      pageUrl: window.location.href,
    }))
    return true
  }

  return false
})
