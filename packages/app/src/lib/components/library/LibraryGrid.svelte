<script lang="ts">
  // Ten thousand images are ten thousand rows, not ten thousand DOM nodes and
  // not ten thousand records. Two things keep that true and both are load
  // bearing: only the rows inside the scroll window are rendered (`gridWindow`,
  // which is where that is tested), and only the `search` pages those rows fall
  // in are fetched.
  import type { ImageRecord, Rating } from '@boorubox/shared'
  import type { SearchResults } from '$lib/api'
  import {
    isTypingTarget,
    KEY_ENTER,
    KEY_INSPECT,
    KEY_SPACE,
  } from '$lib/keyboard'
  import { moveFocus } from './grid-focus'
  import { EDGE, GAP, gridWindow, imageTop } from './grid-window'
  import { groupLabel } from './group-label'
  import ImageCard from './ImageCard.svelte'

  interface Props {
    results: SearchResults
    /** Target tile edge in px; the toolbar slider writes it (design D11). */
    tile: number
    /**
     * The current card, `-1` for none (design D9). Bound so the page can fill
     * the inspector from it — and so the page owns resetting it to `-1` when a
     * new query makes the old index meaningless.
     */
    focusIndex: number
    /**
     * Read-only to the page: the grid is the one place the column count is
     * computed, and the viewer's row step reads it from here (design D9).
     */
    columns: number
    onactivate: (index: number) => void
    onforget: (image: ImageRecord) => void
    onrate: (image: ImageRecord, rating: Rating | null) => void
    ontoggleinspector: () => void
  }

  let {
    results,
    tile,
    focusIndex = $bindable(),
    // eslint-disable-next-line no-useless-assignment -- write-only: published, never read back
    columns = $bindable(),
    onactivate,
    onforget,
    onrate,
    ontoggleinspector,
  }: Props = $props()

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
    if (!focusWanted || focusIndex < 0 || !viewport || mounted === 0) return
    const card = viewport.querySelector<HTMLElement>('[data-card-focus][tabindex="0"]')
    if (!card) return
    card.focus()
    focusWanted = false
  })

  function scrollIntoView(index: number) {
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
    focusIndex = index
    focusWanted = true
    scrollIntoView(index)
  }

  function onkeydown(event: KeyboardEvent) {
    if (isTypingTarget(event)) return

    const destination = moveFocus(
      focusIndex,
      event.key,
      shown.columns,
      results.total,
      results.groups,
    )
    if (destination !== null) {
      event.preventDefault()
      focusCard(destination)
      return
    }

    if (event.key === KEY_INSPECT) {
      event.preventDefault()
      ontoggleinspector()
    } else if (
      (event.key === KEY_ENTER || event.key === KEY_SPACE)
      && focusIndex >= 0
      && focusIndex < results.total
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
            <ImageCard
              image={results.at(index)}
              focused={index === focusIndex}
              onfocus={() => (focusIndex = index)}
              onactivate={() => onactivate(index)}
              onforget={() => {
                const image = results.at(index)
                if (image) onforget(image)
              }}
              onrate={(rating) => {
                const image = results.at(index)
                if (image) onrate(image, rating)
              }}
            />
          {/each}
        </div>
      {/if}
    {/each}
  </div>
</div>
