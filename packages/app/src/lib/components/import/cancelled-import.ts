// The two different re-run sentences the change's proposal calls out
// (`import-pause-cancel`): a bundle resumes because its rows keep the id
// they had in the export, a local import mints a fresh id every run and so
// duplicates everything. Pure text, tested on its own, and shared by
// `ImportReportCard` (both kinds) and the `/import` route (bundle only) so
// the two places a cancelled result is shown never drift into different
// wording for the same kind.

import type { ImportRun } from '$lib/api'

/** What a cancelled run's result says about running it again, by kind. */
export function rerunNotice(kind: ImportRun['kind']): string {
  return kind === 'bundle'
    ? 'Selecting the same parts again will carry on from where it stopped.'
    : 'Importing the same files again will import them again.'
}

/**
 * The queued-runs-discarded sentence (`pending-work` spec "Cancel discards
 * the runs queued behind it"), or `null` for a cancel with nothing queued.
 */
export function queuedDiscardedNotice(count: number): string | null {
  if (count === 0) return null
  return count === 1
    ? '1 queued import was discarded.'
    : `${count.toLocaleString()} queued imports were discarded.`
}

/** The library-switch dialog's title, description and confirm label. */
export interface SwitchConfirmation {
  title: string
  description: string
  confirmLabel: string
}

/**
 * The words for the dialog spec `library-switching` puts in front of every
 * library swap while an import is running or waiting (`import-confirm`
 * design D6). Reuses {@link rerunNotice} for the re-run sentence, so this
 * question and a cancelled report can never claim different things about the
 * same run. The queued clause is its own, in the future tense
 * {@link queuedDiscardedNotice}'s past tense cannot serve — this is asking
 * permission for a discard that has not happened yet, not reporting one that
 * has.
 */
export function switchConfirmation(
  kind: ImportRun['kind'],
  queued: number,
  action: 'close' | 'switch',
): SwitchConfirmation {
  const verb = action === 'close' ? 'Close the library' : 'Switch libraries'
  const queuedClause = queued === 0
    ? ''
    : ` ${queued === 1 ? '1 queued import' : `${queued.toLocaleString()} queued imports`} will be discarded too.`
  return {
    title: `${verb} while an import is running?`,
    description:
      `This cancels the running import. What it has already imported stays in the library `
      + `it imported into. ${rerunNotice(kind)}${queuedClause}`,
    confirmLabel: verb,
  }
}
