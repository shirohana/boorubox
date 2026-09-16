<script lang="ts">
  // A native modal dialog, for the focus behaviour rather than the looks: the
  // browser traps Tab inside it, closes it on Escape, and returns focus to the
  // thumbnail that opened it. Hand-rolling that is how keyboard users get
  // stranded.
  //
  // `←` `→` are BOUNDED, not clamped: at either end nothing happens.
  // `offsetIndexBounded` is that rule; the buttons read it too, so the disabled
  // state and the keys can never disagree. `↑` `↓` are the grid's row step and
  // therefore CLAMPED (design D5) — the asymmetry is deliberate and argued in
  // `navigation-math`.
  import { onDestroy } from 'svelte'
  import type { SearchResults } from '$lib/api'
  import { imageUrl } from '$lib/api'
  import { offsetIndexBounded } from '$lib/domain/navigation-math'
  import {
    isTypingTarget,
    KEY_DOWN,
    KEY_INSPECT,
    KEY_LEFT,
    KEY_RIGHT,
    KEY_SPACE,
    KEY_TAB,
    KEY_UP,
  } from '$lib/keyboard'
  import { moveFocus } from './grid-focus'
  import Inspector from './Inspector.svelte'
  import { nextTabStop } from './tab-cycle'
  import type { TrashActions } from './trash-actions'
  import {
    clickTarget,
    fitScale,
    panOffset,
    wheelZoomFactor,
    zoomAt,
    zoomBy,
    type Point,
    type Size,
  } from './viewer-zoom'

  interface Props {
    results: SearchResults
    index: number
    libraryPath: string | null
    /** The grid's own column count (design D9), so a row step means one grid row. */
    columns: number
    /** Session state, held by the page so it outlives this dialog (design D6). */
    mode: 'gallery' | 'inspect'
    /** Slot Inspector · actions, in this placement of the panel (`trash` design D13). */
    actions: TrashActions
    /** Forwarded to the inspector, which edits here exactly as it does beside the grid. */
    tagQuery: string
    onquery: (next: string, id: string) => void
    /**
     * Every image this viewer moves to. The grid behind it scrolls that index
     * into view, so closing does not jump to a row the user never saw it reach
     * (item 2.4's hand check). It must not move the DOM focus — that stays in
     * this dialog.
     */
    onmove: (index: number) => void
    onclose: () => void
  }

  let {
    results,
    index = $bindable(),
    libraryPath,
    columns,
    mode = $bindable(),
    actions,
    tagQuery,
    onquery,
    onmove,
    onclose,
  }: Props = $props()

  let dialog = $state<HTMLDialogElement | null>(null)
  /** Where the focus lands on open, and not a tab stop (design D2). */
  let surface = $state<HTMLDivElement | null>(null)
  /** The image is fitted into this; the empty space around it closes the viewer (design D4). */
  let stage = $state<HTMLDivElement | null>(null)
  /**
   * The box the image is fitted into and panned inside (design D4), the whole
   * of the stage: its measured size is what `fitScale`/`clickTarget`/
   * `panOffset` see. No margin (`viewer-edge-to-edge` D1): a strip of dark
   * around a covering image made it read as contained, hiding that the edge
   * was cutting it.
   */
  let viewport = $state<HTMLDivElement | null>(null)

  /**
   * `null` reads as "at the fit": the fit itself depends on the natural size
   * of whichever image is showing, so resetting the zoom on `move()` is
   * forgetting the last scale rather than recomputing one (design D3).
   */
  let scale = $state<number | null>(null)
  /**
   * The click's zoom is a frame loop writing `scale`, never a CSS transition
   * (design D7, amended twice). A transition on the size runs on the main
   * thread and one on a transform on the compositor, a frame or two apart, so
   * the top edge of an image the maths keeps still visibly dipped; and any
   * easing on the pan, even 80 ms, reads as the pointer being followed late.
   * Written from one loop, every frame carries the size and the pan the
   * pointer asks for at that instant: the pan is never eased and never cut.
   * `null` while no zoom is in flight.
   */
  let zoomFrame: number | null = null
  /** Set from the `<img>`'s `load` event; `null` until then (see Handoff). */
  let naturalSize = $state<Size | null>(null)
  let viewportWidth = $state(0)
  let viewportHeight = $state(0)
  /** The last pointer position inside the viewport, the pan anchor (design D5). */
  let pointer = $state<Point | null>(null)

  const image = $derived(results.at(index))
  const src = $derived(image && libraryPath ? imageUrl(libraryPath, image) : null)
  const title = $derived(image?.pageTitle || image?.imageUrl || image?.id || '')
  const previous = $derived(offsetIndexBounded(index, -1, results.total))
  const next = $derived(offsetIndexBounded(index, 1, results.total))

  const viewportSize = $derived<Size | null>(
    viewportWidth > 0 && viewportHeight > 0
      ? { width: viewportWidth, height: viewportHeight }
      : null,
  )
  /** Whether the image's natural size and the viewport are both known yet — see Handoff. */
  const measured = $derived(naturalSize !== null && viewportSize !== null)
  const fit = $derived(naturalSize && viewportSize ? fitScale(naturalSize, viewportSize) : 1)
  const displayScale = $derived(scale ?? fit)
  /**
   * Whether the image overflows its viewport — the one question both the
   * click and the pan ask. Not `scale !== null`: a wheel step down clamps to
   * the fit as a *number*, and a click there has to zoom in rather than
   * toggle back to the fit it is already at.
   */
  const zoomed = $derived(displayScale > fit)
  const content = $derived<Size | null>(
    naturalSize
      ? { width: naturalSize.width * displayScale, height: naturalSize.height * displayScale }
      : null,
  )
  const offset = $derived<Point>(
    content && viewportSize && pointer ? panOffset(pointer, viewportSize, content) : { x: 0, y: 0 },
  )
  const imgStyle = $derived(
    content ? `width:${content.width}px;height:${content.height}px;transform:translate(${offset.x}px,${offset.y}px)` : '',
  )

  $effect(() => {
    dialog?.showModal()
    // Design D2: which element inside the dialog gets the focus is pinned here
    // rather than left to the engine's dialog-focusing steps, which differ
    // between WebKit and Chromium. Without it the focus stays in the grid and
    // the arrows below never see a key.
    surface?.focus()
  })

  function move(destination: number | null) {
    if (destination === null) return
    index = destination
    // Design D3: moving on returns the zoom to the fit; the fit itself
    // depends on the new image's natural size, which is not known until its
    // own `load` event.
    stopZoom()
    scale = null
    naturalSize = null
    results.ensureRange(destination, destination + 1)
    onmove(destination)
  }

  function onimgload(event: Event) {
    const el = event.currentTarget as HTMLImageElement
    naturalSize = { width: el.naturalWidth, height: el.naturalHeight }
  }

  function updatePointer(event: { clientX: number, clientY: number }) {
    if (!viewport) return
    const rect = viewport.getBoundingClientRect()
    pointer = { x: event.clientX - rect.left, y: event.clientY - rect.top }
  }

  function stopZoom() {
    if (zoomFrame === null) return
    cancelAnimationFrame(zoomFrame)
    zoomFrame = null
  }

  /**
   * `scale` from where it is to `to` over `ZOOM_EASE_MS`, then `settle`,
   * which is where a zoom back to the fit becomes `null` again — the fit is
   * a number only for the duration of the ride.
   */
  function animateZoom(to: number, settle: () => void) {
    stopZoom()
    const from = displayScale
    const started = performance.now()
    const frame = (now: number) => {
      scale = zoomAt(from, to, now - started)
      if (scale !== to) {
        zoomFrame = requestAnimationFrame(frame)
        return
      }
      zoomFrame = null
      settle()
    }
    zoomFrame = requestAnimationFrame(frame)
  }

  function toggleZoom(event: MouseEvent) {
    updatePointer(event)
    if (!naturalSize || !viewportSize) return
    if (zoomed) animateZoom(fit, () => (scale = null))
    else animateZoom(clickTarget(naturalSize, viewportSize), () => {})
  }

  function onviewportwheel(event: WheelEvent) {
    // Always taken, even before the image is measured: otherwise the page
    // behind scrolls out from under the still-loading picture.
    event.preventDefault()
    if (!naturalSize || !viewportSize) return
    // WebKit reports a trackpad pinch twice: as the gesture events below, whose
    // `scale` is absolute from the gesture's start, and as a ctrl-wheel whose
    // delta is relative. Applied both, every frame would compound the relative
    // step onto the absolute one and the image would jitter — while a gesture
    // is running, the gesture events own the scale.
    if (event.ctrlKey && gestureActive) return
    // The wheel takes over from a click's ride wherever it has got to.
    stopZoom()
    updatePointer(event)
    scale = zoomBy(displayScale, wheelZoomFactor(event), fit)
  }

  function onviewportpointermove(event: PointerEvent) {
    // Only while zoomed: at the fit there is no overflow to pan, so every move
    // would cost a `getBoundingClientRect` (a forced layout) and a style write
    // to arrive back at the same centred image. The gestures that zoom set the
    // anchor from their own event, so nothing is stale on the way in.
    if (!zoomed) return
    updatePointer(event)
  }

  /**
   * WKWebView's pinch (design D8): dispatched as `gesturestart` /
   * `gesturechange` / `gestureend` with a `scale` relative to the gesture's
   * start, on none of which TypeScript's DOM lib knows anything — the shape
   * actually received is declared locally, and the listeners are attached
   * through it rather than through Svelte's typed `on:` attributes.
   */
  interface GestureEvent extends UIEvent {
    scale: number
    clientX: number
    clientY: number
  }

  interface GestureEventTarget {
    addEventListener: (
      type: 'gesturestart' | 'gesturechange' | 'gestureend',
      listener: (event: GestureEvent) => void,
    ) => void
    removeEventListener: (
      type: 'gesturestart' | 'gesturechange' | 'gestureend',
      listener: (event: GestureEvent) => void,
    ) => void
  }

  /** The scale a pinch started from; `event.scale` is relative to it, not to the previous frame. */
  let gestureStartScale = 1
  /** Between `gesturestart` and `gestureend`: the ctrl-wheel of the same pinch is ignored. */
  let gestureActive = false

  function ongesturestart(event: GestureEvent) {
    gestureActive = true
    // WebKit zooms the page itself unless every gesture event is prevented.
    event.preventDefault()
    stopZoom()
    updatePointer(event)
    gestureStartScale = displayScale
  }

  function ongesturechange(event: GestureEvent) {
    event.preventDefault()
    if (!naturalSize || !viewportSize) return
    updatePointer(event)
    scale = zoomBy(gestureStartScale, event.scale, fit)
  }

  function ongestureend(event: GestureEvent) {
    event.preventDefault()
    gestureActive = false
  }

  $effect(() => {
    const target = viewport as unknown as GestureEventTarget | null
    if (!target) return
    target.addEventListener('gesturestart', ongesturestart)
    target.addEventListener('gesturechange', ongesturechange)
    target.addEventListener('gestureend', ongestureend)
    return () => {
      target.removeEventListener('gesturestart', ongesturestart)
      target.removeEventListener('gesturechange', ongesturechange)
      target.removeEventListener('gestureend', ongestureend)
    }
  })

  onDestroy(stopZoom)

  /**
   * The controls Tab may land on, in document order: the inspector's own,
   * while it is showing — nothing else draws a control over the image
   * (design D7). Disabled buttons are not stops, which is the same test the
   * engine makes and the reason the selector spells it out.
   */
  const TAB_STOPS = [
    'a[href]',
    'button:not([disabled])',
    'input:not([disabled])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    '[tabindex]:not([tabindex="-1"])',
  ].join(', ')

  /**
   * `showModal()` is supposed to keep Tab inside the dialog and in WebKit it
   * does not: enough Tabs park the focus on a control of the page behind,
   * where Space is no longer the viewer's (item 2.1's hand check). So the
   * cycle is walked here, over the dialog's own tabbable elements, which is
   * the one form that behaves the same in both engines (design D2, amended).
   *
   * `offsetParent !== null` drops stops that are not currently rendered — the
   * inspector's controls when it is not showing — so a keystroke can't land
   * on one nothing points at.
   */
  function trapTab(event: KeyboardEvent) {
    if (!surface) return
    const stops = [...surface.querySelectorAll<HTMLElement>(TAB_STOPS)]
      .filter((stop) => stop.tabIndex >= 0 && stop.offsetParent !== null)
    const active = document.activeElement
    const from = active instanceof Node
      ? stops.findIndex((stop) => stop === active || stop.contains(active))
      : -1
    const to = nextTabStop(stops.length, from, event.shiftKey)
    if (to === null) return
    event.preventDefault()
    stops[to].focus()
  }

  function onkeydown(event: KeyboardEvent) {
    // Design D3: a control that already acted on this key prevented its
    // default, and the viewer does not act on it a second time — that is what
    // keeps the rating choices' arrows off the image.
    if (event.defaultPrevented) return

    // Ahead of the typing guard on purpose: the tag field is inside the dialog,
    // so Tab out of it is a move between the viewer's controls, not a character
    // being typed. The suggestion list still wins — it prevents Tab's default,
    // which the guard above reads.
    if (event.key === KEY_TAB) {
      trapTab(event)
      return
    }

    if (isTypingTarget(event)) return

    if (event.key === KEY_LEFT) {
      event.preventDefault()
      move(previous)
    } else if (event.key === KEY_RIGHT) {
      event.preventDefault()
      move(next)
    } else if (event.key === KEY_UP || event.key === KEY_DOWN) {
      // The grid's own arithmetic, group slices included (design D5), so a row
      // step cannot mean one thing here and another behind the dialog.
      event.preventDefault()
      move(moveFocus(index, event.key, columns, results.total, results.groups))
    } else if (event.key === KEY_INSPECT) {
      event.preventDefault()
      mode = mode === 'inspect' ? 'gallery' : 'inspect'
    } else if (event.key === KEY_SPACE && (event.target === surface || event.target === dialog)) {
      // Escape is the dialog's own; Space is not, so it has to ask — and only
      // from the two non-controls: the surface the dialog opens focused on, and
      // the dialog itself, which a click on the image or any other unfocusable
      // area leaves as the active element. On a focused button Space is the
      // button's press, which a native button never marks as handled (D4).
      event.preventDefault()
      dialog?.close()
    }
  }

  function onclick(event: MouseEvent) {
    // The second click of a double click on the current tile lands here: the
    // first opened this dialog, which the effect above mounted a microtask
    // later, so without this the tile's own double click closes it again.
    if (event.detail > 1) return
    // Design D4: the dark region is the `::backdrop`, whose clicks target the
    // `<dialog>`, and, inside the viewport box, the space beside a centred
    // image that is not zoomed to fill it — a click there lands on the box
    // itself, not the image. A click on the image or the inspector lands on
    // a descendant and stays there. A covering image leaves nothing to click
    // (`viewer-edge-to-edge`): Escape and Space are the way out then.
    const target = event.target
    if (target === dialog || target === stage || target === viewport) dialog?.close()
  }
</script>

<dialog
  bind:this={dialog}
  {onclose}
  {onkeydown}
  {onclick}
  class="
    m-auto h-screen max-h-none w-screen max-w-none border-0 bg-transparent p-0
    backdrop:bg-black/85
  "
>
  <!--
    Design D2: the viewer's focus holder. `tabindex="-1"` makes it focusable
    without making it a stop in the dialog's tab order, so Shift-Tab from the
    first control reaches the last instead of outlining the whole box.
  -->
  <div bind:this={surface} tabindex="-1" class="flex h-full min-h-0 gap-3 outline-none">
    <div class="flex min-w-0 flex-1 flex-col">
      <div bind:this={stage} class="relative flex min-h-0 min-w-0 flex-1">
        <!--
          The box the image is fitted into and panned inside, edge to edge
          (`viewer-edge-to-edge` D1). A click that lands on this box itself —
          not the image — closes the viewer, same as a click on the stage
          (`onclick` above).
        -->
        <!--
          Wheel, pointer position and the WebKit pinch are mouse/trackpad-only
          by design (Non-Goals: no touch, no keyboard zoom); Tab already
          reaches the inspector's controls without it.
        -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          bind:this={viewport}
          bind:clientWidth={viewportWidth}
          bind:clientHeight={viewportHeight}
          onwheel={onviewportwheel}
          onpointermove={onviewportpointermove}
          class="absolute inset-0 flex items-center justify-center overflow-hidden"
        >
          {#if src}
            <!--
              Undraggable for the same reason as the tile's thumbnail (design
              D1). The click toggling the zoom is mouse-only, same as the div
              above; the image is not a control.
            -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <img
              {src}
              alt={title}
              draggable="false"
              onload={onimgload}
              onclick={toggleZoom}
              class={measured ? 'max-w-none' : 'max-h-full max-w-full object-contain'}
              style={measured ? imgStyle : undefined}
            />
          {:else}
            <p class="text-sm text-white/60">Loading…</p>
          {/if}
        </div>
      </div>
    </div>

    {#if mode === 'inspect'}
      <aside
        class="
          my-2 w-80 shrink-0 overflow-hidden rounded-xl border border-border bg-background
          text-foreground
        "
      >
        <!--
          A rating chosen with Enter or Space leaves the focus on that choice,
          where the next Space is the choice's own press and no longer closes
          the viewer (item 3.1's hand check). Every completed action in the
          panel — a rating, a tag save, a tag or account acted on as a search
          term — says so through `onrelease`, and the focus comes back to the
          surface Space is bound on (`app-frame` design D1, amended). Beside
          the grid the grid's own card gets it instead.
        -->
        <Inspector
          image={image ?? null}
          {results}
          {actions}
          {tagQuery}
          {onquery}
          onrelease={() => surface?.focus()}
        />
      </aside>
    {/if}
  </div>
</dialog>
