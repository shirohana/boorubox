<script lang="ts">
  // Slot Sidebar · filters (design D17), the Collections section (`collections`
  // design D8, `browse-feedback` design D4): every collection with its count in
  // the current result, in name order (spec "The sidebar lists the collections
  // with counts" — an active collection is marked where it is, never moved).
  // Counts ride on `results.counts.collections`, the same round trip the tag
  // list already gets (design D7) — nothing here counts a membership of its
  // own. The rows, though, are the store's list, not the counts: the counts
  // are `null` for the length of every search, and rows drawn from them
  // unmount and remount on each click, which throws the box's scroll
  // position away — the row the user just clicked scrolled out of view. The
  // store's list is stable across searches, and a count that has not arrived
  // is a blank (design D8's honest blank), not a missing row. The fold is a
  // stored preference, the note's own pattern (`NotesPanel.svelte`); the
  // list's height is dragged from its top edge, `SectionResizer`'s
  // (`sidebar-inspector-polish` design D8).
  import type { Collection, CollectionCount } from '@boorubox/shared'
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { collectionDelete, collections, errorText, settings } from '$lib/api'
  import CollectionNameDialog from '$lib/components/common/CollectionNameDialog.svelte'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import SectionResizer from '$lib/components/common/SectionResizer.svelte'
  import { roomAbove } from '$lib/components/common/section-resizer'
  import { Button } from '$lib/components/ui/button'
  import * as Collapsible from '$lib/components/ui/collapsible'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import {
    activeTerms,
    addCollectionToQuery,
    excludeCollectionFromQuery,
    toggleCollectionInQuery,
  } from '$lib/domain/tag-utils'
  import { searchMark } from './categories'
  import CollectionPinMenuItem from './CollectionPinMenuItem.svelte'
  import FilterRow from './FilterRow.svelte'

  interface Props {
    /** `null` while a search is running (design D8): the heading stays, the list is blank. */
    counts: CollectionCount[] | null
    tagQuery: string
    onquery: (next: string) => void
    /**
     * A create, rename or delete changed the list: re-read the counts without
     * touching the query or the selection (design D8) — unlike `onquery`,
     * which is a click on a row and rewrites the search itself.
     */
    onchanged: () => void
  }

  let { counts, tagQuery, onquery, onchanged }: Props = $props()

  /** Follows a window resize, so the drag ceiling (`max` below) does too. */
  let innerHeight = $state(window.innerHeight)

  // The fold is a stored preference, not local state (`browse-feedback`
  // design D4), the same reasoning as the notes panel's own fold.
  const expanded = $derived(!(settings.current?.collectionsCollapsed ?? false))

  // Design D3: the one reader for "is this term active", shared with the tag
  // sidebar and the inspector's badges.
  const terms = $derived(activeTerms(tagQuery))

  /** This search's count per collection id; empty while a search runs. */
  const countById = $derived(new Map((counts ?? []).map((count) => [count.id, count.count])))

  let creating = $state(false)
  let renaming = $state<Collection | null>(null)
  let deleting = $state<Collection | null>(null)
  let error = $state<string | null>(null)

  // The list's height, session-only like the note's (`sidebar-inspector-polish`
  // design D8, `browse-feedback` design D4's session-only reasoning stands):
  // dragged by `SectionResizer` below. 128px, not more: at an 800px window the
  // sidebar's fixed parts already take most of the height, and a taller default
  // starved the tag list to nothing on the first run.
  let height = $state(128)

  const nameDialogOpen = $derived(creating || renaming !== null)

  function closeNameDialog() {
    creating = false
    renaming = null
  }

  /**
   * A create, rename or delete changes two things the screen reads from
   * different places: the counts this section draws (`onchanged`) and the list
   * every menu offers, which is the store's (design D7). Both, or the tile
   * menu keeps offering a collection this row just renamed or deleted.
   */
  function listChanged(): void {
    error = null
    void collections.refresh()
    onchanged()
  }

  function saved(): void {
    closeNameDialog()
    listChanged()
  }

  /**
   * The number named is the row's own, which is the count within the current
   * search (design D7): nothing answers "how many in the library" while a
   * search is on, so the sentence says which number this is rather than
   * implying the other one.
   */
  const deleteDescription = $derived([
    `${(deleting && countById.get(deleting.id)) ?? 0} of the images this search matches are in it.`,
    'Deleting it takes it off every image in it and changes nothing else about them.',
  ].join(' '))

  async function confirmDelete(): Promise<void> {
    const doomed = deleting
    deleting = null
    if (!doomed) return
    try {
      await collectionDelete(doomed.id)
      listChanged()
    } catch (cause) {
      error = errorText(cause)
    }
  }
</script>

<svelte:window bind:innerHeight />

<Collapsible.Root
  open={expanded}
  onOpenChange={(open) => void settings.setCollectionsCollapsed(!open)}
  class="flex flex-col gap-1 p-2"
>
  <!--
    Unfolded only (collections spec "Unfolded, its list SHALL occupy a
    height..."): the handle is the edge the list shares with the tag list
    above it, and there is nothing to drag while the list itself is hidden.
    `max` is a function, not `innerHeight / 2`: that ceiling alone lets
    Collections and Notes each grow to half the window, which together
    overflow the column once the tag list is already at its floor (design
    D8's ceiling amendment). The function reads how much the tag list can
    still give up at drag start, so the two sections' ceilings share one
    budget instead of two independent halves.
  -->
  {#if expanded}
    <SectionResizer
      {height}
      min={40}
      max={() =>
        Math.min(
          innerHeight / 2,
          height + roomAbove(document.querySelector('[data-sidebar="tags"]')),
        )}
      label="Resize the collections list"
      onresize={(next) => (height = next)}
    />
  {/if}

  <div class="flex items-center justify-between px-1 pb-1">
    <Collapsible.Trigger
      class="
        flex items-center gap-1 rounded-md py-0.5 text-xs font-medium text-muted-foreground
        hover:bg-sidebar-accent
      "
    >
      <ChevronDownIcon class="size-3 transition-transform {expanded ? '' : '-rotate-90'}" />
      Collections
    </Collapsible.Trigger>
    <Button
      size="icon"
      variant="ghost"
      class="size-5"
      aria-label="New collection…"
      title="New collection…"
      onclick={() => (creating = true)}
    >
      <PlusIcon class="size-3" />
    </Button>
  </div>

  <Collapsible.Content class="flex flex-col gap-1">
    <!--
      No border, no corner handle: `SectionResizer` above draws the top edge,
      and `max-h-[50vh]` is the CSS ceiling the clamp above mirrors
      (`sidebar-inspector-polish` design D8).
    -->
    <div class="max-h-[50vh] min-h-10 overflow-y-auto" style:height="{height}px">
      {#if collections.list.length === 0}
        <p class="p-1 text-xs text-muted-foreground">No collections</p>
      {:else}
        <ul class="flex flex-col">
          {#each collections.list as collection (collection.id)}
            <!--
              The row reads the same table the tag sidebar does (design D3),
              through `FilterRow` (`sidebar-inspector-polish` design D7): the
              shared emerald tint for an included collection rather than one
              of its own.
            -->
            <FilterRow
              name={collection.name}
              count={countById.get(collection.id) ?? ''}
              mark={searchMark(collection.slug, terms.collections, terms.excludedCollections)}
              oninclude={() => onquery(addCollectionToQuery(tagQuery, collection.slug))}
              onexclude={() => onquery(excludeCollectionFromQuery(tagQuery, collection.slug))}
              ontoggle={() => onquery(toggleCollectionInQuery(tagQuery, collection.slug))}
            >
              {#snippet menu()}
                <CollectionPinMenuItem {collection} />
                <ContextMenu.Separator />
                <ContextMenu.Item onSelect={() => (renaming = collection)}>
                  <PencilIcon />
                  Rename…
                </ContextMenu.Item>
                <ContextMenu.Item variant="destructive" onSelect={() => (deleting = collection)}>
                  <Trash2Icon />
                  Delete…
                </ContextMenu.Item>
              {/snippet}
            </FilterRow>
          {/each}
        </ul>
      {/if}
    </div>

    {#if error}
      <p class="px-1 pt-1 text-xs text-destructive">{error}</p>
    {/if}
  </Collapsible.Content>
</Collapsible.Root>

<CollectionNameDialog
  collection={renaming}
  open={nameDialogOpen}
  onclose={closeNameDialog}
  onsaved={saved}
/>

<!--
  Deleting a collection is not recoverable and takes no image with it (spec
  "Delete"), so this asks and says so — the count is what a click on a row
  whose menu is a moment from view does not otherwise say.
-->
<ConfirmDialog
  title="Delete “{deleting?.name}”?"
  description={deleteDescription}
  confirmLabel="Delete collection"
  open={deleting !== null}
  onclose={() => (deleting = null)}
  onconfirm={confirmDelete}
/>
