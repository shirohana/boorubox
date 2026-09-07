// What the background worker asks a tab for. Both sides import these names, so
// a renamed message cannot compile on one side and silently miss on the other.

import type { PageContext } from './adapters/types.js'
import type { CaptureResult } from './content/capture.js'

export const CAPTURE_IMAGE = 'CAPTURE_IMAGE'

/**
 * The page's context is asked for separately from the bytes (design D9), so a
 * page whose image cannot be canvas-captured still delivers it.
 */
export const EXTRACT_CONTEXT = 'EXTRACT_CONTEXT'

export interface CaptureImageMessage {
  type: typeof CAPTURE_IMAGE
  imageUrl: string
}

export interface ExtractContextMessage {
  type: typeof EXTRACT_CONTEXT
  imageUrl: string
}

export type ContentMessage = CaptureImageMessage | ExtractContextMessage

/** A tab with no content script in it answers nothing at all. */
export type ExtractContextResult = PageContext | undefined

export type { CaptureResult, PageContext }

/** What the popup asks the worker to do; it never posts to the app itself. */
export const RETRY_CAPTURE = 'RETRY_CAPTURE'
export const RETRY_ALL = 'RETRY_ALL'

export interface RetryCaptureMessage {
  type: typeof RETRY_CAPTURE
  id: string
}

export interface RetryAllMessage {
  type: typeof RETRY_ALL
}

export type PopupMessage = RetryCaptureMessage | RetryAllMessage
