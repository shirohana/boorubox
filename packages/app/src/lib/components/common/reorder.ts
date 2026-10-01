// The one reorder primitive, for short lists of uniform rows. A pure `moveItem`, a `dropIndex` that
// turns a pointer position into a destination, and the `reorderable` action
// that wires native HTML5 drag-and-drop onto a list element.
//
// Native drag rather than pointer events: the lists are short and their rows
// uniform, and both webviews (WebKit, WebView2) drag a `draggable` element
// with the OS ghost image for free. The action reorders nothing itself; it
// reports `onmove(from, to)` and the owner writes the new order.
//
// Row contract. The element carrying `use:reorderable` contains the rows;
// every row carries `data-reorder-index="<its index in the list>"`, and
// holds a `ReorderHandle` anywhere inside it. A drag starts only from a
// handle, so text selection and clicks in the rest of the row are untouched.
// `to` is the item's final index in the list after the move, the argument
// `moveItem(list, from, to)` takes.
// While a drag is over a row the row carries `data-reorder-drop="before"` or
// `"after"` for the owner's styling, removed when the drag ends.

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

function rowOf(target: EventTarget | null): HTMLElement | null {
  return target instanceof Element ? target.closest<HTMLElement>(ROW) : null
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

  function onDragStart(event: DragEvent) {
    const handle = event.target instanceof Element ? event.target.closest(HANDLE) : null
    const row = rowOf(handle)
    if (!handle || !row || !event.dataTransfer) return
    dragged = indexOf(row)
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', String(dragged))
    event.dataTransfer.setDragImage?.(row, 0, 0)
  }

  function onDragOver(event: DragEvent) {
    const row = rowOf(event.target)
    if (dragged === null || !row) return
    event.preventDefault()
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move'
    clearMarks()
    row.setAttribute(DROP, pointerInLowerHalf(row, event.clientY) ? 'after' : 'before')
  }

  function onDrop(event: DragEvent) {
    const row = rowOf(event.target)
    const from = dragged
    dragged = null
    clearMarks()
    if (from === null || !row) return
    event.preventDefault()
    const to = dropIndex(from, indexOf(row), pointerInLowerHalf(row, event.clientY))
    if (to !== from) current.onmove(from, to)
  }

  function onDragLeave(event: DragEvent) {
    const into = event.relatedTarget
    if (!(into instanceof Node) || !node.contains(into)) clearMarks()
  }

  function onDragEnd() {
    dragged = null
    clearMarks()
  }

  node.addEventListener('dragstart', onDragStart)
  node.addEventListener('dragover', onDragOver)
  node.addEventListener('drop', onDrop)
  node.addEventListener('dragleave', onDragLeave)
  node.addEventListener('dragend', onDragEnd)
  return {
    update(next: ReorderOptions) {
      current = next
    },
    destroy() {
      node.removeEventListener('dragstart', onDragStart)
      node.removeEventListener('dragover', onDragOver)
      node.removeEventListener('drop', onDrop)
      node.removeEventListener('dragleave', onDragLeave)
      node.removeEventListener('dragend', onDragEnd)
    },
  }
}
