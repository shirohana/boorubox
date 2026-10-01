// The one reorder primitive, for short lists of uniform rows. A pure `moveItem`, a `dropIndex` that
// turns a pointer position into a destination, and the `reorderable` action
// that wires `pointerDrag` onto a list element.
//
// Pointer events, not HTML5 drag: the window's file drop swallows every HTML5
// drag (`api/drag-drop.ts`). The action reorders nothing itself; it reports
// `onmove(from, to)` and the owner writes the new order.
//
// Row contract. The element carrying `use:reorderable` contains the rows;
// every row carries `data-reorder-index="<its index in the list>"`, and
// holds a `ReorderHandle` anywhere inside it. A drag starts only from a
// handle, so text selection and clicks in the rest of the row are untouched.
// `to` is the item's final index in the list after the move, the argument
// `moveItem(list, from, to)` takes.
// While a drag is over a row the row carries `data-reorder-drop="before"` or
// `"after"` for the owner's styling, removed when the drag ends.

import { pointerDrag } from './pointer-drag'

/**
 * A new array with the item at `from` placed at `to`. The same `list` comes
 * back — identity, not a copy — when `from === to` or either index is out of
 * range, so a caller can skip the write on `result === list`.
 */
export function moveItem<T>(list: T[], from: number, to: number): T[] {
  const inRange = (index: number) => Number.isInteger(index) && index >= 0 && index < list.length
  if (from === to || !inRange(from) || !inRange(to)) return list
  const moved = [...list]
  const [item] = moved.splice(from, 1)
  moved.splice(to, 0, item)
  return moved
}

/**
 * The final index of the dragged item after it is dropped on the `before` or
 * `after` half of row `row`: the slot the pointer points at, less one when
 * the item leaves a place above that slot.
 */
export function dropIndex(from: number, row: number, after: boolean): number {
  const slot = after ? row + 1 : row
  return slot > from ? slot - 1 : slot
}

export interface ReorderOptions {
  /** `to` is the item's index after the move (`moveItem`'s argument). */
  onmove: (from: number, to: number) => void
}

const ROW = '[data-reorder-index]'
const HANDLE = '[data-reorder-handle]'
const DROP = 'data-reorder-drop'

function rowAt(x: number, y: number): HTMLElement | null {
  return document.elementFromPoint(x, y)?.closest<HTMLElement>(ROW) ?? null
}

function indexOf(row: HTMLElement): number {
  return Number(row.dataset.reorderIndex)
}

function pointerInLowerHalf(row: HTMLElement, clientY: number): boolean {
  const box = row.getBoundingClientRect()
  return clientY > box.top + box.height / 2
}

export function reorderable(node: HTMLElement, options: ReorderOptions) {
  let current = options
  let dragged: number | null = null

  function clearMarks() {
    for (const row of node.querySelectorAll(`[${DROP}]`)) row.removeAttribute(DROP)
  }

  function markRowAt(x: number, y: number) {
    clearMarks()
    const row = rowAt(x, y)
    if (row && node.contains(row)) row.setAttribute(DROP, pointerInLowerHalf(row, y) ? 'after' : 'before')
  }

  const drag = pointerDrag(node, {
    onstart(x, y, handle) {
      const row = handle.closest<HTMLElement>(ROW)
      dragged = row ? indexOf(row) : null
      markRowAt(x, y)
    },
    onmove: markRowAt,
    onend(x, y, dropped) {
      const from = dragged
      dragged = null
      clearMarks()
      const row = rowAt(x, y)
      if (!dropped || from === null || !row || !node.contains(row)) return
      const to = dropIndex(from, indexOf(row), pointerInLowerHalf(row, y))
      if (to !== from) current.onmove(from, to)
    },
  }, { selector: HANDLE })
  return {
    update(next: ReorderOptions) {
      current = next
    },
    destroy: drag.destroy,
  }
}
