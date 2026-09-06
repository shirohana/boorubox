// How a stored fact is rendered as text. Pure, so the inspector — which has no
// component test harness in this repo — is still covered where the arithmetic
// and the wording live.

import type { Rating } from '@boorubox/shared'

const UNITS = ['B', 'kB', 'MB', 'GB', 'TB']

/** A file size a person reads, from the byte count the row stores. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—'
  let value = bytes
  let unit = 0
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000
    unit += 1
  }
  // Bytes are whole; everything above them reads better to one decimal.
  const digits = unit === 0 || value >= 100 ? 0 : 1
  return `${value.toFixed(digits)} ${UNITS[unit]}`
}

/** An epoch-millisecond column as a local date and time. */
export function formatTimestamp(ms: number): string {
  if (!Number.isFinite(ms)) return '—'
  return new Date(ms).toLocaleString()
}

/**
 * The stored one-letter rating spelled out. `null` is "unrated", which is a
 * fact about the image and not a missing value: `is:unrated` searches for it.
 */
export function ratingLabel(rating: Rating | null): string {
  if (rating === null) return 'unrated'
  return { g: 'general', s: 'sensitive', q: 'questionable', e: 'explicit' }[rating]
}
