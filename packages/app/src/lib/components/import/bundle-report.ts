// What `/import` renders a bundle report as (design D7): failed and skipped
// named individually — they are what the user has to act on — imported left
// as a count, so a report of thousands of rows stays readable.

import type { ImageCounts, ImportOutcome, ImportReport } from '@boorubox/shared'

/**
 * One status's share of a report: the count from the report's own summary
 * field, and its items for the list. Kept apart because the route previews
 * only the first `OUTCOME_PREVIEW_LIMIT` items — `count` stays the true total
 * once `items` is sliced for display.
 */
export interface BundleOutcomeGroup {
  count: number
  items: ImportOutcome[]
}

export interface BundleReportGroups {
  failed: BundleOutcomeGroup
  skipped: BundleOutcomeGroup
  imported: number
}

export function groupBundleReport(report: ImportReport): BundleReportGroups {
  return {
    failed: { count: report.failed, items: report.items.filter((item) => item.status === 'failed') },
    skipped: {
      count: report.skipped,
      items: report.items.filter((item) => item.status === 'skipped'),
    },
    imported: report.imported,
  }
}

/**
 * The counts line beside the report (design D7, spec `legacy-bundle-import`
 * "Counts shown"): total and `source=legacy-bundle` — library-wide, not this
 * run's own count, since a bundle moves over in parts and `legacyBundle`
 * counts every one ever imported. Worded after §9 step 4 itself ("the count
 * imported from the extension"), not "from this bundle".
 */
export function bundleCountsLine(counts: ImageCounts): string {
  return `${counts.total.toLocaleString()} images total, `
    + `${counts.legacyBundle.toLocaleString()} imported from the extension.`
}

/** §9 step 4's notice, verbatim — the app never says the browser is safe to clear. */
export const BUNDLE_MIGRATION_NOTICE
  = 'Compare these numbers and spot-check a few images before removing anything from the '
    + 'browser. This app cannot verify the browser\'s data for you.'

/**
 * Shown for a bundle run that has started but not yet reported its first
 * `import:progress` — Rust counts the parts' rows before importing any
 * (design D5). Shared with `ImportRunTile`, which shows the same run on the
 * library band, so the two never drift into different wording for one state.
 */
export const BUNDLE_COUNTING_LABEL = 'Counting the bundle\'s rows…'
