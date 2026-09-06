<script lang="ts">
  import type { ImageCounts, ImageRecord } from '@boorubox/shared'
  import {
    dropImageRecord,
    errorText,
    imageCounts,
    library,
    SearchResults,
    type SearchInputs,
  } from '$lib/api'
  import CountsPanel from '$lib/components/library/CountsPanel.svelte'
  import EmptyState from '$lib/components/library/EmptyState.svelte'
  import ImportPanel from '$lib/components/library/ImportPanel.svelte'
  import LibraryGrid from '$lib/components/library/LibraryGrid.svelte'
  import Lightbox from '$lib/components/library/Lightbox.svelte'
  import ListenerBanner from '$lib/components/library/ListenerBanner.svelte'
  import SearchBar from '$lib/components/library/SearchBar.svelte'

  const results = new SearchResults()

  let counts = $state<ImageCounts | null>(null)
  let lightboxIndex = $state(0)
  let lightboxOpen = $state(false)
  let actionError = $state<string | null>(null)

  const libraryPath = $derived(library.status?.libraryPath ?? null)

  async function loadCounts() {
    try {
      counts = await imageCounts()
    } catch (error) {
      actionError = errorText(error)
    }
  }

  function runSearch(inputs: SearchInputs) {
    lightboxOpen = false
    void results.run(inputs)
  }

  /** Design D16: the record goes, the file under `images/` stays. */
  async function forget(image: ImageRecord) {
    try {
      library.set(await dropImageRecord(image.id))
      await Promise.all([results.refresh(), loadCounts()])
    } catch (error) {
      actionError = errorText(error)
    }
  }

  void results.run({ tagQuery: '', text: '' })
  void loadCounts()
</script>

<main class="flex h-screen flex-col">
  <ListenerBanner listener={library.status?.listener ?? null} />

  <header class="flex flex-col gap-3 border-b border-border px-4 py-3">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <CountsPanel {counts} />
      <ImportPanel
        onimported={() => {
          void results.refresh()
          void loadCounts()
        }}
      />
    </div>
    <SearchBar onsearch={runSearch} />
    {#if results.loading && results.total > 0}
      <p role="status" class="text-xs text-muted-foreground">Searching…</p>
    {/if}
    {#if actionError}
      <p class="text-xs text-destructive">{actionError}</p>
    {/if}
  </header>

  <div class="min-h-0 flex-1">
    {#if results.error}
      <div class="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
        <p class="text-sm font-medium">The search could not be run.</p>
        <p class="text-sm text-muted-foreground">{results.error}</p>
      </div>
    {:else if results.total === 0}
      <!--
        `total` holds its last answer while a search runs, so the grid is only
        replaced when there is genuinely nothing to show. Swapping it out on
        every refresh would remount it and throw away the scroll position.
      -->
      {#if results.loading}
        <p class="p-8 text-sm text-muted-foreground">Searching…</p>
      {:else}
        <EmptyState inputs={results.inputs} />
      {/if}
    {:else}
      <LibraryGrid
        {results}
        onactivate={(index) => {
          lightboxIndex = index
          lightboxOpen = true
        }}
        onforget={forget}
      />
    {/if}
  </div>
</main>

{#if lightboxOpen}
  <Lightbox
    {results}
    {libraryPath}
    bind:index={lightboxIndex}
    onclose={() => (lightboxOpen = false)}
  />
{/if}
