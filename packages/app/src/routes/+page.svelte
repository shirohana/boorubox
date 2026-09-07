<script lang="ts">
  import type { ImageRecord } from '@boorubox/shared'
  import { GRID_TILE_DEFAULT, GRID_TILE_MAX, GRID_TILE_MIN } from '@boorubox/shared'
  import PanelRightIcon from '@lucide/svelte/icons/panel-right'
  import {
    dropImageRecord,
    errorText,
    imports,
    library,
    onCaptureStored,
    onFileDrop,
    SearchResults,
    settings,
    type SearchInputs,
  } from '$lib/api'
  import { frame } from '$lib/components/frame/frame.svelte'
  import EmptyState from '$lib/components/library/EmptyState.svelte'
  import ImportMenu from '$lib/components/library/ImportMenu.svelte'
  import ImportReportCard from '$lib/components/library/ImportReportCard.svelte'
  import Inspector from '$lib/components/library/Inspector.svelte'
  import LibraryGrid from '$lib/components/library/LibraryGrid.svelte'
  import Lightbox from '$lib/components/library/Lightbox.svelte'
  import PendingBand from '$lib/components/library/PendingBand.svelte'
  import SearchBar from '$lib/components/library/SearchBar.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Slider } from '$lib/components/ui/slider'
  import { isTypingTarget, KEY_SEARCH } from '$lib/keyboard'

  const results = new SearchResults()

  const noQuery: SearchInputs = { tagQuery: '', text: '' }
  let inputs = $state<SearchInputs>(noQuery)
  /** The current card, `-1` for none; the inspector shows whatever it names. */
  let focusIndex = $state(-1)
  /** Session state, not a setting (design D9 / Non-Goals): open until hidden. */
  let inspectorOpen = $state(true)
  // The grid follows the drag; only the release writes the setting (design D11),
  // so this is the live edge and `gridTileSize` is where it comes back from.
  let tile = $state(settings.current?.gridTileSize ?? GRID_TILE_DEFAULT)
  let lightboxIndex = $state(0)
  let lightboxOpen = $state(false)
  let grid = $state<LibraryGrid | null>(null)
  let hovering = $state(false)
  let actionError = $state<string | null>(null)

  const libraryPath = $derived(library.status?.libraryPath ?? null)
  const focused = $derived(results.at(focusIndex) ?? null)

  function runSearch(next: SearchInputs) {
    inputs = next
    // A new list: the old index names a different image, or none at all.
    focusIndex = -1
    lightboxOpen = false
    void results.run(next)
  }

  // Spec `library-switching`: everything reading the library follows a switch,
  // and the switch is started from the sidebar footer, which this page cannot
  // hear about any other way than by the path it holds changing.
  let shownPath = library.status?.libraryPath ?? null
  $effect(() => {
    const path = library.status?.libraryPath ?? null
    if (path === shownPath) return
    shownPath = path
    focusIndex = -1
    lightboxOpen = false
    // The reports describe runs into the library that was open, not this one.
    imports.dismissAll()
    void results.run(inputs)
  })

  // A capture from the browser extension lands in the library with nothing on
  // this screen having asked for it. Without this the grid keeps the result set
  // it last searched, and the image only appears once something else re-runs the
  // search — leaving and coming back to the route, for instance.
  $effect(() => {
    const subscription = onCaptureStored(() => void results.refresh())
    subscription.catch((cause) => (actionError = errorText(cause)))
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  // The runs outlive this route (design D9), so what they change is told to
  // whoever is on screen rather than to whoever started them.
  $effect(() => imports.onfinished(() => void results.refresh()))

  // Design D8: the subscription is the route's, not the Import menu's. A menu is
  // unmounted while its dropdown is closed, which is nearly always — registered
  // in there, drop-to-import would silently stop working.
  $effect(() => {
    const subscription = onFileDrop({
      onhover: () => (hovering = true),
      onleave: () => (hovering = false),
      ondrop: (paths) => {
        hovering = false
        imports.enqueue(paths)
      },
    })
    subscription.catch((cause) => (actionError = errorText(cause)))
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  /**
   * The keyboard map's one frame-wide binding. It lives on this route and not in
   * the layout because the field it focuses is this toolbar's: bound in the
   * layout it would fire on /settings, where `/` would do nothing. It moves to
   * the layout the day a second screen has a search field.
   */
  function focusSearch(event: KeyboardEvent) {
    if (event.key !== KEY_SEARCH || lightboxOpen || isTypingTarget(event)) return
    const field = document.getElementById('tag-query')
    if (!(field instanceof HTMLInputElement)) return
    event.preventDefault()
    field.focus()
  }

  /** Design D16: the record goes, the file under `images/` stays. */
  async function forget(image: ImageRecord) {
    try {
      library.set(await dropImageRecord(image.id))
      await results.refresh()
    } catch (error) {
      actionError = errorText(error)
    }
  }

  // The route's controls in the frame's top bar, for as long as this route is
  // mounted (see `frame.svelte.ts`).
  $effect(() => {
    frame.toolbar = toolbar
    return () => {
      frame.toolbar = null
    }
  })

  void results.run(noQuery)
</script>

<svelte:window onkeydown={focusSearch} />

{#snippet toolbar()}
  <SearchBar onsearch={runSearch} />

  <Slider
    type="single"
    class="w-24 shrink-0"
    aria-label="Thumbnail size"
    min={GRID_TILE_MIN}
    max={GRID_TILE_MAX}
    step={10}
    bind:value={tile}
    onValueCommit={(size) => {
      settings.setGridTileSize(size).catch((error) => (actionError = errorText(error)))
    }}
  />

  <!-- The variant, not just `aria-pressed`: the state has to be visible. -->
  <Button
    size="icon-sm"
    variant={inspectorOpen ? 'secondary' : 'ghost'}
    aria-label="Show or hide the inspector"
    aria-pressed={inspectorOpen}
    onclick={() => (inspectorOpen = !inspectorOpen)}
  >
    <PanelRightIcon />
  </Button>

  <ImportMenu />
{/snippet}

{#each imports.reports as report (report)}
  <ImportReportCard {report} ondismiss={() => imports.dismiss(report)} />
{/each}

{#if actionError || imports.error}
  <p class="border-b border-border px-4 py-2 text-xs text-destructive">
    {actionError ?? imports.error}
  </p>
{/if}

<div class="flex min-h-0 flex-1">
  <div class="flex min-w-0 flex-1 flex-col">
    <!-- Above everything the results area can be: the grid, the empty library
         and the search that found nothing (spec `pending-work`). -->
    <PendingBand {tile} />

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
          bind:this={grid}
          {results}
          {tile}
          bind:focusIndex
          onactivate={(index) => {
            lightboxIndex = index
            lightboxOpen = true
          }}
          onforget={forget}
          ontoggleinspector={() => (inspectorOpen = !inspectorOpen)}
        />
      {/if}
    </div>
  </div>

  {#if inspectorOpen}
    <aside class="w-80 shrink-0 border-s border-border">
      <Inspector image={focused} />
    </aside>
  {/if}
</div>

{#if lightboxOpen}
  <Lightbox
    {results}
    {libraryPath}
    bind:index={lightboxIndex}
    onclose={() => {
      lightboxOpen = false
      // The native dialog has just put the focus back on the card it opened
      // from; the viewer may have moved on since, and the grid follows it.
      grid?.focusCard(lightboxIndex)
    }}
  />
{/if}

{#if hovering}
  <div
    class="
      pointer-events-none fixed inset-0 z-40 flex items-center justify-center bg-background/80
    "
  >
    <p class="rounded-xl border border-dashed border-border px-6 py-4 text-sm">
      Drop images or folders to import them
    </p>
  </div>
{/if}
