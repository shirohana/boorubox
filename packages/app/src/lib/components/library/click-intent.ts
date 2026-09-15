// Whether a click on the viewer's image is the single-click chrome toggle or
// the double-click zoom (design D2). Pure: the decision from a click's
// `detail` and whether a single click is still pending, so the double-click
// window — and the "a lone second click means nothing" case — are asserted
// here instead of hunted for in a running dialog. The component owns the
// timer; this module owns what the timer's outcome means.

/** The platforms' default double-click interval, before the single-click action fires. */
export const DOUBLE_CLICK_MS = 250

export type ClickIntent
  /** `detail === 1`: start (or restart) the single-click timer. */
  = 'schedule'
    /** `detail === 2` while a single click is pending: cancel it, this is the double click. */
    | 'double'
    /** `detail === 2` with nothing pending: the first click landed elsewhere, on the tile. */
    | 'ignore'

export function clickIntent(detail: number, pending: boolean): ClickIntent {
  if (detail <= 1) return 'schedule'
  return pending ? 'double' : 'ignore'
}
