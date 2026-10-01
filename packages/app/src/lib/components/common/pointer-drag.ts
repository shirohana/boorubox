// A drag driven by pointer events. Inside this window HTML5 drag never reaches
// the page: Tauri's drag-drop handler (kept on for import by drop, see
// `api/drag-drop.ts`) takes every drag first. Use this for any drag between
// elements of the page.
//
// `pointerDrag(root, handlers, options)` arms on a primary-button pointer-down
// on `root`, or on the `options.selector` match inside it (delegation, for
// handles that come and go). The pointer is captured at once; a move past
// `THRESHOLD` px starts the drag. Under capture every event is routed to the
// handle, so the consumer finds what is under the pointer itself with
// `document.elementFromPoint(x, y)`, which reads the hit-test tree, not the
// event route.

export const THRESHOLD = 4

export interface PointerDragHandlers {
  /** The pointer moved past the threshold; `handle` is the element pressed. */
  onstart?: (x: number, y: number, handle: HTMLElement) => void
  /** Every move after the start, in client coordinates. */
  onmove?: (x: number, y: number) => void
  /**
   * The drag ended, once. `dropped` is true only for a pointer-up; a
   * pointercancel, Escape or a capture lost to a blur end with `false`.
   */
  onend?: (x: number, y: number, dropped: boolean) => void
}

export interface PointerDragOptions {
  /** Arm only on a pointer-down whose target is inside a match of this selector. */
  selector?: string
}

export function pointerDrag(
  root: HTMLElement,
  handlers: PointerDragHandlers,
  options: PointerDragOptions = {},
) {
  let current = handlers
  let armed: { handle: HTMLElement, pointerId: number, x: number, y: number } | null = null
  let started = false
  let last = { x: 0, y: 0 }

  function finish(x: number, y: number, dropped: boolean) {
    if (!armed) return
    const { handle, pointerId } = armed
    const wasStarted = started
    armed = null
    started = false
    window.removeEventListener('keydown', onKeyDown, true)
    if (handle.hasPointerCapture?.(pointerId) !== false) handle.releasePointerCapture?.(pointerId)
    if (wasStarted) current.onend?.(x, y, dropped)
  }

  function onPointerDown(event: PointerEvent) {
    if (armed || event.button !== 0 || !(event.target instanceof Element)) return
    const handle = options.selector ? event.target.closest<HTMLElement>(options.selector) : root
    if (!handle || !root.contains(handle)) return
    armed = { handle, pointerId: event.pointerId, x: event.clientX, y: event.clientY }
    last = { x: event.clientX, y: event.clientY }
    handle.setPointerCapture?.(event.pointerId)
    window.addEventListener('keydown', onKeyDown, true)
  }

  function onPointerMove(event: PointerEvent) {
    if (!armed || event.pointerId !== armed.pointerId) return
    if (!started) {
      if (Math.hypot(event.clientX - armed.x, event.clientY - armed.y) <= THRESHOLD) return
      started = true
      current.onstart?.(event.clientX, event.clientY, armed.handle)
    }
    last = { x: event.clientX, y: event.clientY }
    current.onmove?.(event.clientX, event.clientY)
  }

  function onPointerUp(event: PointerEvent) {
    if (armed && event.pointerId === armed.pointerId) finish(event.clientX, event.clientY, true)
  }

  function onPointerCancel(event: PointerEvent) {
    if (armed && event.pointerId === armed.pointerId) finish(event.clientX, event.clientY, false)
  }

  function onLostCapture(event: PointerEvent) {
    if (armed && event.pointerId === armed.pointerId) finish(event.clientX, event.clientY, false)
  }

  // Capture on `window`, and stopped once the drag has started: an Escape that
  // reached a dialog's own listener would close it and lose the drag's ticks.
  function onKeyDown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !armed) return
    if (started) event.stopPropagation()
    finish(last.x, last.y, false)
  }

  root.addEventListener('pointerdown', onPointerDown)
  root.addEventListener('pointermove', onPointerMove)
  root.addEventListener('pointerup', onPointerUp)
  root.addEventListener('pointercancel', onPointerCancel)
  root.addEventListener('lostpointercapture', onLostCapture)
  return {
    update(next: PointerDragHandlers) {
      current = next
    },
    destroy() {
      finish(0, 0, false)
      root.removeEventListener('pointerdown', onPointerDown)
      root.removeEventListener('pointermove', onPointerMove)
      root.removeEventListener('pointerup', onPointerUp)
      root.removeEventListener('pointercancel', onPointerCancel)
      root.removeEventListener('lostpointercapture', onLostCapture)
    },
  }
}

/** Distance from a scroll box's top or bottom edge inside which a held drag scrolls it. */
const EDGE_BAND = 40

/** Pixels per frame at `distance` from the edge: faster the nearer the pointer is to it. */
const edgeSpeed = (distance: number) => 2 + 14 * (1 - Math.max(0, distance) / EDGE_BAND)

function scrollsVertically(element: Element): boolean {
  if (element.scrollHeight <= element.clientHeight) return false
  const { overflowY } = getComputedStyle(element)
  return overflowY === 'auto' || overflowY === 'scroll'
}

// The box is whatever scrolls under the pointer, never a configured region: the
// nearest ancestor of the element at the pointer that overflows and has an
// `overflow-y` of `auto` or `scroll`, else the document's scrolling element.
// Resolved afresh on every call and frame, so a host needs no prop and a box
// that appears or disappears under a held pointer is followed.
function scrollBoxAt(x: number, y: number): Element | null {
  for (let element = document.elementFromPoint(x, y); element; element = element.parentElement) {
    if (scrollsVertically(element)) return element
  }
  return document.scrollingElement
}

/** The box's visible vertical extent; the viewport for the document's scrolling element. */
function visibleExtent(box: Element): { top: number, bottom: number } {
  if (box === document.scrollingElement) return { top: 0, bottom: window.innerHeight }
  return box.getBoundingClientRect()
}

/** The signed scroll step for a pointer at `y` in `box`, 0 outside both edge bands. */
function edgeStep(box: Element, y: number): number {
  const { top, bottom } = visibleExtent(box)
  if (y < top + EDGE_BAND) return -edgeSpeed(y - top)
  if (y > bottom - EDGE_BAND) return edgeSpeed(bottom - y)
  return 0
}

/**
 * Scrolls whatever scrolls under a held drag pointer. Call `at(x, y)` on every
 * drag move and `stop()` when the drag ends. While the pointer sits in the top
 * or bottom band of the box under it, one step per frame moves its `scrollTop`
 * until the step moves nothing or the pointer leaves the band. `onscroll`
 * runs after each step, for a host whose drop target moves under a still pointer.
 */
export function edgeScroller(onscroll?: () => void) {
  let frame: number | null = null
  let pointer = { x: 0, y: 0 }

  function step() {
    frame = null
    const box = scrollBoxAt(pointer.x, pointer.y)
    const delta = box ? edgeStep(box, pointer.y) : 0
    if (!box || delta === 0) return
    const before = box.scrollTop
    box.scrollTop += delta
    onscroll?.()
    if (box.scrollTop !== before) arm()
  }

  function arm() {
    if (frame === null) frame = requestAnimationFrame(step)
  }

  return {
    at(x: number, y: number) {
      pointer = { x, y }
      const box = scrollBoxAt(x, y)
      if (box && edgeStep(box, y) !== 0) arm()
    },
    stop() {
      if (frame !== null) cancelAnimationFrame(frame)
      frame = null
    },
  }
}
