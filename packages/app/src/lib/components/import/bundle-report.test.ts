import type { ImportReport } from '@boorubox/shared'
import { expect, it } from 'vitest'
import { BUNDLE_MIGRATION_NOTICE, bundleCountsLine, groupBundleReport } from './bundle-report'

const report: ImportReport = {
  imported: 2,
  skipped: 1,
  failed: 1,
  cancelled: false,
  items: [
    { path: 'https://example/1', status: 'imported', id: 'a' },
    { path: 'https://example/2', status: 'imported', id: 'b' },
    { path: 'https://example/3', status: 'skipped', id: 'c', reason: 'already in the library' },
    { path: 'https://example/bad.db', status: 'failed', reason: 'not a SQLite database' },
  ],
}

it('splits a report into the three groups the route renders', () => {
  expect(groupBundleReport(report)).toEqual({
    imported: 2,
    skipped: {
      count: 1,
      items: [
        { path: 'https://example/3', status: 'skipped', id: 'c', reason: 'already in the library' },
      ],
    },
    failed: {
      count: 1,
      items: [
        { path: 'https://example/bad.db', status: 'failed', reason: 'not a SQLite database' },
      ],
    },
  })
})

it('names nothing for a clean import', () => {
  const clean: ImportReport = { imported: 3, skipped: 0, failed: 0, cancelled: false, items: [] }
  expect(groupBundleReport(clean)).toEqual({
    imported: 3,
    skipped: { count: 0, items: [] },
    failed: { count: 0, items: [] },
  })
})

it('takes counts from the report summary, not from a (possibly trimmed) items list', () => {
  // A report whose `items` disagree with its own summary — the summary wins,
  // since the route slices `items` for display and must not let that slicing
  // change the headline count.
  const trimmed: ImportReport = {
    imported: 1,
    skipped: 500,
    failed: 0,
    cancelled: false,
    items: [{ path: 'https://example/1', status: 'skipped', id: 'a', reason: 'already in the library' }],
  }
  expect(groupBundleReport(trimmed).skipped.count).toBe(500)
  expect(groupBundleReport(trimmed).skipped.items).toHaveLength(1)
})

it('formats the counts line from total and legacy-bundle counts, worded after §9', () => {
  expect(bundleCountsLine({ total: 3569, extension: 3000, local: 369, legacyBundle: 200 }))
    .toBe('3,569 images total, 200 imported from the extension.')
})

it('states the §9 notice verbatim, never that the browser is safe to clear', () => {
  expect(BUNDLE_MIGRATION_NOTICE).toBe(
    'Compare these numbers and spot-check a few images before removing anything from the '
    + 'browser. This app cannot verify the browser\'s data for you.',
  )
  expect(BUNDLE_MIGRATION_NOTICE.toLowerCase()).not.toContain('safe')
})
