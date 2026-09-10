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
  import { moveFocus } from './grid-focus'
  import Inspector from './Inspector.svelte'
  import { nextTabStop } from './tab-cycle'
  import type { TrashActions } from './trash-actions'

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
    onquery: (next: string) => void
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

  const image = $derived(results.at(index))
  const src = $derived(image && libraryPath ? imageUrl(libraryPath, image) : null)
  const title = $derived(image?.pageTitle || image?.imageUrl || image?.id || '')
  const previous = $derived(offsetIndexBounded(index, -1, results.total))
  const next = $derived(offsetIndexBounded(index, 1, results.total))

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
    results.ensureRange(destination, destination + 1)
    onmove(destination)
  }

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
   */
  function trapTab(event: KeyboardEvent) {
    if (!surface) return
    const stops = [...surface.querySelectorAll<HTMLElement>(TAB_STOPS)]
      .filter((stop) => stop.tabIndex >= 0)
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
    // `<dialog>`, and the empty space around the image inside the transparent
    // box. A click on either lands on that element itself; a click on the
    // image, the chrome or the inspector lands on a descendant and stays there.
    if (event.target === dialog || event.target === stage) dialog?.close()
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
      <!-- Minimal chrome: the image is what the viewer is for. -->
      <!-- The dialog reaches the window's top edge, where the traffic lights are (D13). -->
      <header
        class="
          flex items-center justify-between gap-3 px-1 py-2 text-white
          in-data-[platform=macos]:ps-16
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
            size="sm"
            variant="ghost"
            class="text-white hover:bg-white/15 hover:text-white"
            disabled={previous === null}
            onclick={() => move(previous)}
          >
            Previous
          </Button>
          <Button
            size="sm"
            variant="ghost"
            class="text-white hover:bg-white/15 hover:text-white"
            disabled={next === null}
            onclick={() => move(next)}
          >
            Next
          </Button>
          <Button
            size="sm"
            variant="ghost"
            class="text-white hover:bg-white/15 hover:text-white"
            aria-pressed={mode === 'inspect'}
            onclick={() => (mode = mode === 'inspect' ? 'gallery' : 'inspect')}
          >
            Info
          </Button>
          <Button
            size="sm"
            variant="ghost"
            class="text-white hover:bg-white/15 hover:text-white"
            onclick={() => dialog?.close()}
          >
            Close
          </Button>
        </div>
      </header>

      <!--
        The image is fitted to whatever space is left, so opening the inspector
        refits it rather than cropping it (design D10).
      -->
      <div bind:this={stage} class="flex min-h-0 min-w-0 flex-1 items-center justify-center">
        {#if src}
          <!-- Undraggable for the same reason as the tile's thumbnail (design D1). -->
          <img
            {src}
            alt={title}
            draggable="false"
            class="max-h-full max-w-full object-contain"
          />
        {:else}
          <p class="text-sm text-white/60">Loading…</p>
        {/if}
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
          the viewer (item 3.1's hand check). The panel says when a choice was
          made and the focus comes back to the surface Space is bound on
          (design D4, amended). Beside the grid nothing is passed, so the
          control keeps its focus there.
        -->
        <Inspector
          image={image ?? null}
          {results}
          {actions}
          {tagQuery}
          {onquery}
          onrated={() => surface?.focus()}
        />
      </aside>
    {/if}
  </div>
</dialog>
