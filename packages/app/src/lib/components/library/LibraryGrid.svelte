<script lang="ts">
  // Ten thousand images are ten thousand rows, not ten thousand DOM nodes and
  // not ten thousand records. Two things keep that true and both are load
  // bearing: only the rows inside the scroll window are rendered (`gridWindow`,
  // which is where that is tested), and only the `search` pages those rows fall
  // in are fetched.
  import type { ImageRecord } from '@boorubox/shared'
  import type { SearchResults } from '$lib/api'
  import { CARD_HEIGHT, EDGE, GAP, gridWindow } from './grid-window'
  import ImageCard from './ImageCard.svelte'

  interface Props {
    results: SearchResults
    onactivate: (index: number) => void
    onforget: (image: ImageRecord) => void
  }

  let { results, onactivate, onforget }: Props = $props()

  let viewport = $state<HTMLDivElement | null>(null)
  let scrollTop = $state(0)
  let viewportWidth = $state(0)
  let viewportHeight = $state(0)

  const shown = $derived(gridWindow({
    total: results.total,
    scrollTop,
    width: viewportWidth,
    height: viewportHeight,
  }))
  const windowRows = $derived(
    Array.from({ length: shown.endIndex - shown.firstIndex }, (_, i) => shown.firstIndex + i),
  )

  // `ensureRange` reads the result generation itself, so a new query re-runs
  // this effect and the window is asked for again.
  $effect(() => {
    results.ensureRange(shown.firstIndex, shown.endIndex)
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
</script>

<div
  bind:this={viewport}
  bind:clientWidth={viewportWidth}
  bind:clientHeight={viewportHeight}
  onscroll={(event) => (scrollTop = event.currentTarget.scrollTop)}
  class="h-full overflow-y-auto"
  style="padding: {EDGE}px"
>
  <div class="relative" style="height: {shown.contentHeight}px">
    <div
      class="absolute inset-x-0 grid"
      style="
        top: {shown.offsetTop}px;
          gap: {GAP}px;
          grid-template-columns: repeat({shown.columns}, minmax(0, 1fr));
      "
    >
      {#each windowRows as index (index)}
        <ImageCard
          image={results.at(index)}
          height={CARD_HEIGHT}
          onactivate={() => onactivate(index)}
          onforget={() => {
            const image = results.at(index)
            if (image) onforget(image)
          }}
        />
      {/each}
    </div>
  </div>
</div>
