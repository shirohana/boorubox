<script lang="ts">
  // Ten thousand images are ten thousand rows, not ten thousand DOM nodes and
  // not ten thousand records. Two things keep that true and both are load
  // bearing: only the rows inside the scroll window are rendered (`gridWindow`,
  // which is where that is tested), and only the `search` pages those rows fall
  // in are fetched.
  import type { ImageRecord, Rating } from '@boorubox/shared'
  import type { SearchResults, Selection } from '$lib/api'
  import CollectionNameDialog from '$lib/components/common/CollectionNameDialog.svelte'
  import {
    isTrashKey,
    isTypingTarget,
    KEY_ENTER,
    KEY_INSPECT,
    KEY_SPACE,
  } from '$lib/keyboard'
  import type { CollectionTarget } from './collection-actions'
  import { addToCreated } from './collection-actions'
  import { moveFocus } from './grid-focus'
  import { EDGE, GAP, gridWindow, imageTop } from './grid-window'
  import { groupLabel } from './group-label'
  import ImageCard from './ImageCard.svelte'
  import type { TrashActions } from './trash-actions'

  interface Props {
    results: SearchResults
    /** Target tile edge in px; the toolbar slider writes it (design D11). */
    tile: number
    /**
     * The focus, the anchor and the selected set, which are one state machine
     * (design D1): the grid reads them and never keeps a second copy. The page
     * fills the inspector from the same store and resets it on a new query.
     */
    selection: Selection
    /**
     * Read-only to the page: the grid is the one place the column count is
     * computed, and the viewer's row step reads it from here (design D9).
     */
    columns: number
    /** Offered by every tile's menu, and by the trash keys (`trash` D12, D13). */
    actions: TrashActions
    onactivate: (index: number) => void
    onrate: (image: ImageRecord, rating: Rating | null) => void
    ontoggleinspector: () => void
    /** Reported like every other action failure on this screen (`collections` design D8). */
    onerror: (message: string) => void
  }

  let {
    results,
    tile,
    selection,
    // eslint-disable-next-line no-useless-assignment -- write-only: published, never read back
    columns = $bindable(),
    actions,
    onactivate,
    onrate,
    ontoggleinspector,
    onerror,
  }: Props = $props()

  /**
   * A tile's collection write answers with the records it changed, the same
   * `replace` a tag or rating save uses (design D8/D10) — `results` is
   * already in scope here, so no tile needs it as a prop of its own. In one
   * pass: the write may have named the whole selection.
   */
  function onCollectionsWritten(records: ImageRecord[]): void {
    results.replaceMany(records)
  }

  /**
   * The tile that chose "New collection…", or `null` while no tile has. One
   * dialog for the whole grid, mounted outside every tile's context menu:
   * bits-ui unmounts a closed menu's content, and a `Dialog` per tile would
   * be machinery on the path the grid mounts and destroys tiles on as it
   * scrolls. `$state.raw` because a target is four closures, not data to proxy.
   */
  let creatingCollectionFor = $state.raw<CollectionTarget | null>(null)

  let viewport = $state<HTMLDivElement | null>(null)
  let scrollTop = $state(0)
  let viewportWidth = $state(0)
  let viewportHeight = $state(0)

  const shown = $derived(gridWindow({
    total: results.total,
    groups: results.groups,
    scrollTop,
    width: viewportWidth,
    height: viewportHeight,
    tile,
  }))

  // `ensureRange` reads the result generation itself, so a new query re-runs
  // this effect and the window is asked for again.
  $effect(() => {
    results.ensureRange(shown.firstIndex, shown.endIndex)
  })

  // Published rather than recomputed by the page (design D9): a second
  // computation would drift the day the gap or the padding changes.
  $effect(() => {
    columns = shown.columns
  })

  // A new query is a new list; staying at the old scroll offset would show its
  // middle with no way to tell that is what happened. A plain refresh is the
  // same list, so it keeps its position — hence `queryGeneration`, not
  // `generation`.
  let scrolledForQuery = -1
  $effect(() => {
    if (viewport && scrolledForQuery !== results.queryGeneration) {
      scrolledForQuery = results.queryGeneration
      viewport.scrollTop = 0
    }
  })

  // A key moved the focus, so the DOM has to catch up: the destination row may
  // be outside the window and therefore not mounted yet. Scrolling comes first,
  // the row mounts on the scroll, and this effect then finds the card. Only a
  // key sets it — a click already put the focus where the user aimed, and
  // stealing it back would fight the pointer.
  let focusWanted = $state(false)

  $effect(() => {
    // Reading the window makes this run again once the row it names is mounted.
    const mounted = shown.rows.length
    if (!focusWanted || selection.focus < 0 || !viewport || mounted === 0) return
    const card = currentCard()
    if (!card) return
    card.focus()
    focusWanted = false
  })

  // A resize, a sidebar toggle or a slider drag that changes the column count
  // re-keys every row but the first (`t${row.first}`), and the card holding
  // the focus leaves the DOM with its row: the focus falls to `<body>`, where
  // none of the grid's keys are bound — which is how leaving macOS fullscreen
  // left the keyboard dead. `$effect.pre` runs before the DOM catches up, while
  // the card is still there to ask; the effect above then finds its
  // replacement once the new rows are mounted.
  $effect.pre(() => {
    if (shown.columns === 0 || selection.focus < 0) return
    if (viewport?.contains(document.activeElement)) focusWanted = true
  })

  /** Brings a card the keyboard moved to on screen, without touching the anchor. */
  function showCard(index: number) {
    focusWanted = true
    scrollIntoView(index)
  }

  /**
   * Also the viewer's way in: every image it moves to is scrolled to here, so
   * the grid behind the dialog is looking at the same row and closing does not
   * jump to one the user never saw it reach (item 2.4's hand check). It moves
   * no focus — the viewer's dialog keeps that.
   */
  export function scrollIntoView(index: number) {
    if (!viewport) return
    const { columns, rowHeight, sections } = shown
    // Group headings push every row below them down, so the offset comes from
    // the layout rather than from the index alone (design D7).
    const top = EDGE + imageTop(sections, index, columns, rowHeight)
    const bottom = top + rowHeight - GAP

    if (top < viewport.scrollTop) viewport.scrollTop = top - EDGE
    else if (bottom > viewport.scrollTop + viewport.clientHeight) {
      viewport.scrollTop = bottom + EDGE - viewport.clientHeight
    }
  }

  /**
   * Also the page's way in: the lightbox hands the index it closed on here, so
   * the grid's focus follows what the viewer showed last (design D10).
   */
  export function focusCard(index: number) {
    selection.focusAt(index)
    showCard(index)
  }

  /** The card holding the roving tab stop, or `null` while its row is not mounted. */
  function currentCard(): HTMLElement | null {
    return viewport?.querySelector<HTMLElement>('[data-card-focus][tabindex="0"]') ?? null
  }

  /**
   * Any click that leaves no control focused hands the keyboard back here
   * (`app-frame` design D1, widened by `browse-fixes` design D4: a completed
   * action in the inspector was the first case, and the panel's plain text,
   * an address or empty space anywhere on the screen orphan the DOM focus the
   * same way). Only the DOM focus moved, not the selection, so this brings
   * the focused card back into it without touching `selection.focusAt` —
   * that would drag the anchor along behind a write that never meant to move
   * it. `preventScroll` while the card is mounted: a click that only orphaned
   * the focus must not also jump the scroll. A wheel scroll never moves
   * `selection.focus`, so the current card can have left the window
   * (`grid-window`), and then there is nothing mounted to focus: that case
   * goes through `showCard`, which scrolls the row in and lets the effect
   * above focus it once it exists.
   */
  export function refocus(): void {
    if (selection.focus < 0 || !viewport) return
    const card = currentCard()
    if (card) card.focus({ preventScroll: true })
    else showCard(selection.focus)
  }

  /**
   * The selection if there is one, otherwise the focused image (`trash` design
   * D12). Two or more images are confirmed before anything moves, by the screen
   * that owns the dialog — this key handler asks for the write and nothing
   * else. The focus index is left where it is, but the card under it goes with
   * the write's refresh — the screen puts the focus back on that row afterwards
   * (`LibraryScreen`'s `afterWrite`), which is what lets `Delete` be pressed
   * twice.
   */
  async function trashFocused() {
    if (selection.count > 0) {
      actions.trash(await selection.ids())
      return
    }
    const id = results.at(selection.focus)?.id
    if (id !== undefined) actions.trash([id])
  }

  function onkeydown(event: KeyboardEvent) {
    // Design D5: one handler and one guard for every key the grid binds, the
    // selection keys included. A second dispatcher would have to work out which
    // region is live, which is the thing focus already answers.
    //
    // Select-all is the exception and is bound by the screen (design D5,
    // amended): it is about the result, not about the card the focus is on, and
    // bound here it did nothing until the grid had been clicked into.
    if (isTypingTarget(event)) return
    // The tile checkbox handles Space/Enter itself (bits-ui prevents default
    // without stopping propagation), so a checkbox toggle must not also open
    // the viewer.
    if (event.defaultPrevented) return
    const focusIndex = selection.focus

    if (isTrashKey(event, results.view)) {
      event.preventDefault()
      void trashFocused()
      return
    }

    const destination = moveFocus(
      focusIndex,
      event.key,
      shown.columns,
      results.total,
      results.groups,
    )
    if (destination !== null) {
      event.preventDefault()
      // The destination is the clamped index a plain arrow would take, so a
      // shift-arrow stops at the same edges rather than growing past them.
      if (event.shiftKey) selection.extendTo(destination)
      else selection.focusAt(destination)
      showCard(destination)
      return
    }

    if (event.key === KEY_INSPECT) {
      event.preventDefault()
      ontoggleinspector()
    } else if (
      (event.key === KEY_ENTER || event.key === KEY_SPACE)
      && focusIndex >= 0
      && focusIndex < results.total
      // From the card itself, not from a control on it: Enter on a tile's own
      // button (trash, the menu) activates that button, and opening the
      // viewer as well would be the checkbox's Space bug in another coat.
      && event.target instanceof HTMLElement
      && event.target.hasAttribute('data-card-focus')
    ) {
      event.preventDefault()
      onactivate(focusIndex)
    }
  }
</script>

<!--
  The keys are handled on the scroll container rather than on each card: they
  bubble from whichever card holds the roving tab stop, and `i` still works when
  the focus is on the container itself.
-->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  bind:this={viewport}
  bind:clientWidth={viewportWidth}
  bind:clientHeight={viewportHeight}
  onscroll={(event) => (scrollTop = event.currentTarget.scrollTop)}
  {onkeydown}
  tabindex="-1"
  class="h-full overflow-y-auto outline-none"
  style="padding: {EDGE}px"
>
  <div class="relative" style="height: {shown.contentHeight}px">
    {#each shown.rows as row (row.kind === 'heading' ? `h${row.key}` : `t${row.first}`)}
      {#if row.kind === 'heading'}
        <!--
          The heading and its count come from the group slices, so both are right
          before the page holding the group's images has arrived (design D7).
        -->
        <h2
          class="
            absolute inset-x-0 flex items-end gap-2 truncate pb-1 text-xs font-medium
            text-muted-foreground
          "
          style="top: {row.top}px; height: {shown.headingHeight}px"
        >
          <span class="truncate">{groupLabel(results.group, row.key)}</span>
          <span class="tabular-nums">{row.count.toLocaleString()}</span>
        </h2>
      {:else}
        <div
          class="absolute inset-x-0 grid"
          style="
            top: {row.top}px;
              gap: {GAP}px;
              grid-template-columns: repeat({shown.columns}, minmax(0, 1fr));
          "
        >
          {#each Array.from({ length: row.count }, (_, i) => row.first + i) as index (index)}
            {@const image = results.at(index)}
            <ImageCard
              {image}
              focused={index === selection.focus}
              selected={selection.has(index, image?.id)}
              onfocus={() => selection.focusEntered(index)}
              onselect={(modifiers) => void selection.click(index, image?.id, modifiers)}
              ontoggle={() => {
                if (image) void selection.toggle(index, image.id)
              }}
              onactivate={() => onactivate(index)}
              onrate={(rating) => {
                if (image) onrate(image, rating)
              }}
              view={results.view}
              {actions}
              {selection}
              onwritten={onCollectionsWritten}
              {onerror}
              onnewcollection={(target) => (creatingCollectionFor = target)}
            />
          {/each}
        </div>
      {/if}
    {/each}
  </div>
</div>

<!--
  One dialog for the grid, outside every tile's menu, raised with the target of
  whichever tile asked for it — see `collection-actions.ts` for why it cannot
  live in the menu, and design D8 for why creating here also adds.
-->
<CollectionNameDialog
  collection={null}
  open={creatingCollectionFor !== null}
  onclose={() => (creatingCollectionFor = null)}
  onsaved={(collection) => {
    const target = creatingCollectionFor
    creatingCollectionFor = null
    if (target) void addToCreated(target, collection)
  }}
/>
