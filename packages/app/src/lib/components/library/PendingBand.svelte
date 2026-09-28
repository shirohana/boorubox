<script lang="ts">
  // The row that is about to exist (design D7). It is above the grid's scroll
  // container rather than rows inside it, because every index in the grid is a
  // search index the inspector, the lightbox and the keyboard map all share:
  // prepending rows would shift all of them, and a placeholder inside a
  // filtered search would claim to match a query it was never tested against.
  //
  // Outside the scroll container also means it stays in view while the user
  // scrolls, which is what "the thing I just asked for is coming" needs.
  //
  // Which is also why the band's tile edge is capped at the grid's default
  // (owner, 2026-09-28): a band that stays in view at a 640 tile is most of
  // the window, so above the default the band keeps one row of default-size
  // tiles rather than following the zoom. Below it the tiles are the grid's
  // and line up with its first row, as before.
  import { GRID_TILE_DEFAULT } from '@boorubox/shared'
  import { imports, pendingCaptures, sidecarsBackfill } from '$lib/api'
  import { columnsFor, EDGE, GAP } from './grid-window'
  import ImportRunTile from './ImportRunTile.svelte'
  import PendingCaptureTile from './PendingCaptureTile.svelte'
  import SidecarsTile from './SidecarsTile.svelte'

  /** The grid's tile edge; the band's tiles are the grid's tiles up to the default. */
  let { tile }: { tile: number } = $props()

  let width = $state(0)

  const hasQueuedWork = $derived(imports.runs.length > 0 || pendingCaptures.entries.length > 0)
  const showing = $derived(hasQueuedWork || sidecarsBackfill.progress !== null)
  const bandTile = $derived(Math.min(tile, GRID_TILE_DEFAULT))
  // The same module the grid lays its rows out with, at the same width: the
  // wrapper is measured unpadded and the column count takes `EDGE` off itself,
  // exactly as the grid's scroll container does.
  const columns = $derived(columnsFor(width, bandTile))
</script>

<!-- Measured even while empty, so the first tile is laid out on a real width. -->
<div bind:clientWidth={width}>
  {#if showing}
    <div
      class="grid"
      style="
        padding: {EDGE}px {EDGE}px 0;
          gap: {GAP}px;
          grid-template-columns: repeat({columns}, minmax(0, 1fr));
      "
    >
      <SidecarsTile />
      {#each imports.runs as run (run.id)}
        <ImportRunTile {run} />
      {/each}
      {#each pendingCaptures.entries as capture (capture.id)}
        <PendingCaptureTile {capture} />
      {/each}
    </div>
  {/if}
</div>
