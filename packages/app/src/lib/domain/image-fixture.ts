// Test-only builder: every domain test needs a full ImageRecord to override two
// fields of. Nothing outside *.test.ts should import it.

import type { ImageRecord } from '@boorubox/shared'

export function img(overrides: Partial<ImageRecord> = {}): ImageRecord {
  return {
    id: 'id',
    ext: 'png',
    mime: 'image/png',
    size: 1000,
    width: 100,
    height: 100,
    source: 'extension',
    sourceRef: null,
    imageUrl: 'https://example.com/a.png',
    pageUrl: 'https://example.com/page',
    pageTitle: null,
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
    ...overrides,
  }
}
