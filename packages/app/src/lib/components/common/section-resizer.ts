// The pure math behind dragging a sidebar section's top edge
// (`sidebar-inspector-polish` design D8), split from `SectionResizer.svelte`
// so it can be tested without a DOM (the shape of `library/tile-click.ts`).

/**
 * `start`'s height after the pointer has moved `delta` px, clamped to
 * `[min, max]`. The handle sits on the section's *top* edge, so `delta` is
 * positive when the pointer moved up — dragging up grows the section below
 * it. A `NaN` `start` (no drag has begun) reads as `min`, ignoring `delta`.
 */
export function clampHeight(start: number, delta: number, min: number, max: number): number {
  if (Number.isNaN(start)) return min
  return Math.min(max, Math.max(min, start + delta))
}

/**
 * How far `list` (the tag list, `data-sidebar="tags"`) could shrink from its
 * current height down to its CSS floor — the room a section above it may grow
 * into (`sidebar-inspector-polish` design D8's ceiling amendment). The floor
 * is read from the computed `min-height` rather than a constant so `min-h-32`
 * on the tag list stays the one definition. A missing or non-numeric
 * `min-height` and a `null` list both read as no room, never negative.
 */
export function roomAbove(list: Element | null): number {
  if (!list) return 0
  const minHeight = parseFloat(getComputedStyle(list).minHeight)
  return Math.max(0, list.clientHeight - (Number.isFinite(minHeight) ? minHeight : 0))
}
