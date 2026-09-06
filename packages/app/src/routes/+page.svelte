<script lang="ts">
  import type { ImageRecord, Rating } from '@boorubox/shared'
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
  import RatingPills from '$lib/components/tags/RatingPills.svelte'
  import TagSidebar from '$lib/components/tags/TagSidebar.svelte'
  import EmptyState from '$lib/components/library/EmptyState.svelte'
  import ImportMenu from '$lib/components/library/ImportMenu.svelte'
  import ImportReportCard from '$lib/components/library/ImportReportCard.svelte'
  import Inspector from '$lib/components/library/Inspector.svelte'
  import LibraryGrid from '$lib/components/library/LibraryGrid.svelte'
  import Lightbox from '$lib/components/library/Lightbox.svelte'
  import PendingBand from '$lib/components/library/PendingBand.svelte'
  import SearchBar from '$lib/components/library/SearchBar.svelte'
  import ViewControls from '$lib/components/library/ViewControls.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Slider } from '$lib/components/ui/slider'
  import { isTypingTarget, KEY_SEARCH } from '$lib/keyboard'

  const results = new SearchResults()

  const noQuery: SearchInputs = { tagQuery: '', text: '' }
  // The query lives here, not in the search bar: the sidebar, the rating pills
  // and the inspector rewrite it too (design D14), and the field has to show
  // what ran.
  let tagQuery = $state('')
  let text = $state('')
  const inputs = $derived<SearchInputs>({ tagQuery, text })
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
    tagQuery = next.tagQuery
    text = next.text
    // A new list: the old index names a different image, or none at all.
    focusIndex = -1
    lightboxOpen = false
    void results.run(next)
  }

  /** Every tag and rating click rewrites the query and runs it (design D14). */
  const searchFor = (next: string) => runSearch({ tagQuery: next, text })

  /** Slot Grid · tile: the menu rates its own image, not the inspector's. */
  async function rate(image: ImageRecord, rating: Rating | null) {
    try {
      await results.saveRating(image.id, rating)
    } catch (error) {
      actionError = errorText(error)
    }
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

  // The route's controls in the frame's top bar and its filters region, for as
  // long as this route is mounted (see `frame.svelte.ts`). The sidebar's
  // filters are absent rather than empty on every other screen, which is what
  // unsetting them here means.
  $effect(() => {
    frame.toolbar = toolbar
    frame.filters = filters
    return () => {
      frame.toolbar = null
      frame.filters = null
    }
  })

  void results.run(noQuery)
</script>

<svelte:window onkeydown={focusSearch} />

{#snippet toolbar()}
  <SearchBar bind:tagQuery bind:text onsearch={runSearch} />

  <ViewControls
    sort={results.sort}
    group={results.group}
    onsort={(sort) => {
      focusIndex = -1
      void results.setSort(sort)
    }}
    ongroup={(group) => {
      focusIndex = -1
      void results.setGroup(group)
    }}
  />

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

{#snippet filters()}
  <!--
    Slot Sidebar · filters. Both panels stay mounted while a search runs:
    unmounting them flickers the whole region on every click. Blank counts
    (`null`) are still honest (design D8) — the last query's numbers never
    show, only their own headings do until the new counts arrive.
  -->
  <RatingPills counts={results.counts?.ratings ?? null} {tagQuery} onquery={searchFor} />
  <TagSidebar tags={results.counts?.tags ?? null} {tagQuery} onquery={searchFor} />
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
          onrate={rate}
          ontoggleinspector={() => (inspectorOpen = !inspectorOpen)}
        />
      {/if}
    </div>
  </div>

  {#if inspectorOpen}
    <aside class="w-80 shrink-0 border-s border-border">
      <Inspector image={focused} {results} {tagQuery} onquery={searchFor} />
    </aside>
  {/if}
</div>

{#if lightboxOpen}
  <Lightbox
    {results}
    {libraryPath}
    {tagQuery}
    onquery={searchFor}
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
