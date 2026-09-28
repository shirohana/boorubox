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

interface SizeFacts {
  size: number
  file: string
  width: number
  height: number
}

/**
 * The inspector's Size row, Danbooru's form (`inspector-facts-relabel`
 * design D2): bytes, the stored file's extension, pixel dimensions. The
 * extension is read from `file`'s own name rather than `mime`, so it reads
 * `.jpg` and not `image/jpeg`, and is omitted — with its leading space —
 * when the name has none.
 */
export function formatSizeLine({ size, file, width, height }: SizeFacts): string {
  const name = file.slice(file.lastIndexOf('/') + 1)
  const dot = name.lastIndexOf('.')
  const ext = dot === -1 ? '' : name.slice(dot + 1).toLowerCase()
  const dimensions = `(${width}×${height})`
  return ext ? `${formatBytes(size)} .${ext} ${dimensions}` : `${formatBytes(size)} ${dimensions}`
}

const MINUTE = 60 * 1000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

/** Largest unit first, so the first one whose magnitude is at least one wins. */
const RELATIVE_UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365 * DAY],
  ['month', 30 * DAY],
  ['day', DAY],
  ['hour', HOUR],
  ['minute', MINUTE],
]

const relativeFormat = new Intl.RelativeTimeFormat('en', { numeric: 'auto' })

/**
 * How long ago an epoch-millisecond column reads (`inspector-facts-relabel`
 * design D3), Danbooru's form: `yesterday`, `3 days ago`, `2 years ago`,
 * computed once against `now` rather than ticking. `now` defaults to the
 * call time and is a parameter so a test can fix it.
 */
export function formatRelative(ms: number, now: number = Date.now()): string {
  if (!Number.isFinite(ms)) return '—'
  const diff = ms - now
  for (const [unit, unitMs] of RELATIVE_UNITS) {
    const value = diff / unitMs
    if (Math.abs(value) >= 1) return relativeFormat.format(Math.round(value), unit)
  }
  return 'just now'
}
