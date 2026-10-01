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
