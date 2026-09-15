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
  import { flushSync, onDestroy } from 'svelte'
  import type { SearchResults } from '$lib/api'
  import { imageUrl } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
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
  import { clickIntent, DOUBLE_CLICK_MS } from './click-intent'
  import { moveFocus } from './grid-focus'
  import Inspector from './Inspector.svelte'
  import { nextTabStop } from './tab-cycle'
  import type { TrashActions } from './trash-actions'
  import { fitScale, panOffset, zoomStep, zoomTarget, type Point, type Size } from './viewer-zoom'

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
   * The inset box the image is fitted into and panned inside (design D4). Its
   * measured size, not the stage's, is what `fitScale`/`zoomTarget`/`panOffset`
   * see, so the margin around the image (Tailwind's `inset-6`, 24px — `design
   * D4`'s `PAN_MARGIN_PX`) is never itself part of the picture.
   */
  let viewport = $state<HTMLDivElement | null>(null)

  /** Shown or hidden by a click on the image or by Tab; reset per mount, i.e. per open (D1). */
  let chrome = $state(false)
  /**
   * `null` reads as "at the fit": the fit itself depends on the natural size
   * of whichever image is showing, so resetting the zoom on `move()` is
   * forgetting the last scale rather than recomputing one (design D3).
   */
  let scale = $state<number | null>(null)
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
   * double click and the pan ask. Not `scale !== null`: a wheel step down
   * clamps to the fit as a *number*, and a double click there has to zoom in
   * rather than toggle back to the fit it is already at.
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

  function toggleZoom(event: MouseEvent) {
    updatePointer(event)
    if (zoomed) {
      scale = null
    } else if (naturalSize && viewportSize) {
      scale = zoomTarget(naturalSize, viewportSize)
    }
  }

  function onviewportwheel(event: WheelEvent) {
    // Always taken, even before the image is measured: otherwise the page
    // behind scrolls out from under the still-loading picture.
    event.preventDefault()
    // A trackpad's horizontal swipe reports `deltaY === 0`: no notch in either
    // direction. Read as one, a sideways swipe shrinks the zoom.
    if (event.deltaY === 0) return
    if (!naturalSize || !viewportSize) return
    updatePointer(event)
    scale = zoomStep(displayScale, event.deltaY < 0 ? 1 : -1, fit)
  }

  function onviewportpointermove(event: PointerEvent) {
    // Only while zoomed: at the fit there is no overflow to pan, so every move
    // would cost a `getBoundingClientRect` (a forced layout) and a style write
    // to arrive back at the same centred image. The gestures that zoom set the
    // anchor from their own event, so nothing is stale on the way in.
    if (!zoomed) return
    updatePointer(event)
  }

  /** Design D2: a click's `detail` and whether one is pending decide the gesture. */
  let singleClickTimer: ReturnType<typeof setTimeout> | null = null

  function onimageclick(event: MouseEvent) {
    const intent = clickIntent(event.detail, singleClickTimer !== null)
    if (intent === 'schedule') {
      // A click far enough from the last one restarts the engine's `detail`
      // count, so a second `detail === 1` can arrive while one is pending:
      // without this its timer is unreachable and the bar toggles twice, a
      // flash 250ms apart.
      if (singleClickTimer !== null) clearTimeout(singleClickTimer)
      singleClickTimer = setTimeout(() => {
        singleClickTimer = null
        chrome = !chrome
      }, DOUBLE_CLICK_MS)
    } else if (intent === 'double') {
      if (singleClickTimer !== null) clearTimeout(singleClickTimer)
      singleClickTimer = null
      toggleZoom(event)
    }
  }

  onDestroy(() => {
    if (singleClickTimer !== null) clearTimeout(singleClickTimer)
  })

  /**
   * The controls Tab may land on, in document order: `Previous`, `Next`,
   * `Info`, `Close` and, while it is showing, the inspector's own. Disabled
   * buttons are not stops — `Previous` at the first image is one — which is the
   * same test the engine makes and the reason the selector spells it out.
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
   * does not: five Tabs from the viewer's chrome park the focus on a control of
   * the page behind, where Space is no longer the viewer's (item 2.1's hand
   * check). So the cycle is walked here, over the dialog's own tabbable
   * elements, which is the one form that behaves the same in both engines
   * (design D2, amended).
   *
   * `offsetParent !== null` drops stops inside the hidden chrome (design D1):
   * the bar is hidden with the `hidden` attribute rather than opacity for
   * exactly this — `display: none` clears `offsetParent`, so its buttons
   * leave the cycle the same keystroke that would otherwise land on them.
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
      // Design D6: Tab reveals the chrome before it moves the focus, so the
      // first Tab lands on Previous rather than on a bar that is not there
      // yet. `flushSync` forces the `hidden` attribute off before `trapTab`
      // reads `offsetParent` on the same keystroke.
      if (!chrome) {
        chrome = true
        flushSync()
      }
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
    // `<dialog>`; the margin between the stage's edge and the viewport box;
    // and, inside the viewport box, the space beside a centred image that is
    // not zoomed to fill it — the box forwards those the same way, because a
    // click there lands on the box itself, not the image. A click on the
    // image, the chrome or the inspector lands on a descendant and stays there.
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
    m-auto h-[96vh] max-h-none w-[96vw] max-w-none border-0 bg-transparent p-0
    backdrop:bg-black/85
  "
>
  <!--
    Design D2: the viewer's focus holder. `tabindex="-1"` makes it focusable
    without making it a stop in the dialog's tab order, so Shift-Tab from
    `Previous` reaches the last control instead of outlining the whole box.
  -->
  <div bind:this={surface} tabindex="-1" class="flex h-full min-h-0 gap-3 outline-none">
    <div class="flex min-w-0 flex-1 flex-col">
      <!--
        The image is fitted to the whole stage (design D1): the chrome floats
        over it in the corner instead of taking a row of its own, and the
        dialog reaches the window's top edge, where the traffic lights are
        (D13) — cleared by shifting the bar, not by padding a header that no
        longer spans the width.
      -->
      <div bind:this={stage} class="relative flex min-h-0 min-w-0 flex-1">
        <!--
          The inset box the image is fitted into and panned inside; the strip
          between this and the stage's edge is the margin (design D4). A click
          that lands on this box itself — not the image — closes the viewer,
          same as a click on the stage (`onclick` above).
        -->
        <!--
          Wheel and pointer position are mouse-only by design (Non-Goals: no
          touch, no keyboard zoom); Tab already reaches the chrome without it.
        -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          bind:this={viewport}
          bind:clientWidth={viewportWidth}
          bind:clientHeight={viewportHeight}
          onwheel={onviewportwheel}
          onpointermove={onviewportpointermove}
          class="absolute inset-6 flex items-center justify-center overflow-hidden"
        >
          {#if src}
            <!--
              Undraggable for the same reason as the tile's thumbnail (design
              D1). The click toggling the chrome or the zoom is mouse-only,
              same as the div above; the image is not a control.
            -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <img
              {src}
              alt={title}
              draggable="false"
              onload={onimgload}
              onclick={onimageclick}
              class={measured ? 'max-w-none' : 'max-h-full max-w-full object-contain'}
              style={measured ? imgStyle : undefined}
            />
          {:else}
            <p class="text-sm text-white/60">Loading…</p>
          {/if}
        </div>

        <!--
          The bar (design D1): title, counter and the four buttons, hidden by
          default and shown by a click on the image (via `onimageclick`,
          design D2) or by Tab. `hidden`, not opacity — see `trapTab`.
        -->
        <header
          hidden={!chrome}
          class="
            absolute top-2 left-2 z-10 flex max-w-[calc(100%-1rem)] items-center gap-3 rounded-lg
            bg-black/70 px-2.5 py-1.5 text-white backdrop-blur-sm
            in-data-[platform=macos]:left-16 in-data-[platform=macos]:max-w-[calc(100%-4.5rem)]
          "
        >
          <div class="min-w-0">
            <p class="truncate text-sm">{title}</p>
            <p class="text-xs text-white/60">
              {(index + 1).toLocaleString()} of {results.total.toLocaleString()}
            </p>
          </div>
          <div class="flex shrink-0 items-center gap-1">
            <Button
              size="xs"
              variant="ghost"
              class="text-white hover:bg-white/15 hover:text-white"
              disabled={previous === null}
              onclick={() => move(previous)}
            >
              Previous
            </Button>
            <Button
              size="xs"
              variant="ghost"
              class="text-white hover:bg-white/15 hover:text-white"
              disabled={next === null}
              onclick={() => move(next)}
            >
              Next
            </Button>
            <Button
              size="xs"
              variant="ghost"
              class="text-white hover:bg-white/15 hover:text-white"
              aria-pressed={mode === 'inspect'}
              onclick={() => (mode = mode === 'inspect' ? 'gallery' : 'inspect')}
            >
              Info
            </Button>
            <Button
              size="xs"
              variant="ghost"
              class="text-white hover:bg-white/15 hover:text-white"
              onclick={() => dialog?.close()}
            >
              Close
            </Button>
          </div>
        </header>
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
