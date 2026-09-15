// Test-only builder: every domain test needs a full ImageRecord to override two
// fields of. Nothing outside *.test.ts should import it.

import type { ImageRecord } from '@boorubox/shared'

export function img(overrides: Partial<ImageRecord> = {}): ImageRecord {
  const merged: ImageRecord = {
    id: 'id',
    ext: 'png',
    file: '',
    mime: 'image/png',
    size: 1000,
    width: 100,
    height: 100,
    source: 'extension',
    sourceRef: null,
    imageUrl: 'https://example.com/a.png',
    pageUrl: 'https://example.com/page',
    pageTitle: null,
    account: null,
    adapter: null,
    rating: null,
    tags: [],
    capturedAt: 0,
    fileModifiedAt: null,
    createdAt: 0,
    updatedAt: 0,
    deletedAt: null,
    missing: false,
    posts: [],
    collections: [],
    ...overrides,
  }
  // `file` is opaque to the webview (design D2: Rust composes it, `imageUrl`
  // reads it verbatim), so a fixture only has to keep it consistent with the
  // id that ended up in `merged` — not mirror the bucket rule.
  return overrides.file === undefined
    ? { ...merged, file: `images/${merged.id}.${merged.ext}` }
    : merged
}
