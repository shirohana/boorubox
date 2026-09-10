// Where Tab moves inside a region the focus must not leave. Pure, and separate
// from the component, so the wrap at both ends is asserted here instead of
// being hunted for in a running dialog.

/**
 * The index Tab (or Shift-Tab, `backwards`) moves to among `count` stops, or
 * `null` when there is nothing to move to.
 *
 * The cycle WRAPS: the last stop leads back to the first, which is what makes
 * it a trap rather than a walk. `from` is `-1` when the focus is not on a stop
 * at all — the viewer's focus surface is the case — and then Tab takes the
 * first stop and Shift-Tab the last, so both directions enter the cycle from
 * the end the user is reaching for.
 */
export function nextTabStop(count: number, from: number, backwards: boolean): number | null {
  if (count <= 0) return null
  if (from < 0 || from >= count) return backwards ? count - 1 : 0
  return (from + (backwards ? -1 : 1) + count) % count
}
