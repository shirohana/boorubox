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
  // is a blank (design D8's honest blank), not a missing row. The fold and
  // the list's height are the note's own pattern (`NotesPanel.svelte`): a
  // stored preference and a `resize-y` box.
  import type { Collection, CollectionCount } from '@boorubox/shared'
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down'
  import MinusIcon from '@lucide/svelte/icons/minus'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import { collectionDelete, collections, errorText, settings } from '$lib/api'
  import CollectionNameDialog from '$lib/components/common/CollectionNameDialog.svelte'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Collapsible from '$lib/components/ui/collapsible'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import {
    activeTerms,
    addCollectionToQuery,
    excludeCollectionFromQuery,
    toggleCollectionInQuery,
  } from '$lib/domain/tag-utils'

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

<Collapsible.Root
  open={expanded}
  onOpenChange={(open) => void settings.setCollectionsCollapsed(!open)}
  class="flex flex-col gap-1 p-2"
>
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
      `h-32`, not more: at an 800px window the sidebar's fixed parts already
      take most of the height, and a taller default here starved the tag list
      to nothing on the first run. The user drags the corner for more.
    -->
    <div class="
      h-32 max-h-[50vh] min-h-10 resize-y overflow-y-auto rounded-md border border-border
    ">
      {#if collections.list.length === 0}
        <p class="p-1 text-xs text-muted-foreground">No collections</p>
      {:else}
        <ul class="flex flex-col gap-0.5 p-0.5">
          {#each collections.list as collection (collection.id)}
            {@const active = terms.collections.has(collection.slug)}
            {@const excluded = terms.excludedCollections.has(collection.slug)}
            <li>
              <ContextMenu.Root>
                <ContextMenu.Trigger>
                  {#snippet child({ props })}
                    <div
                      {...props}
                      class="
                        flex items-center gap-1 rounded-md px-1 text-xs
                        hover:bg-sidebar-accent
                        {active
                          ? `bg-emerald-500/15 font-medium text-emerald-700 dark:text-emerald-300`
                          : ''}
                        {excluded ? 'bg-destructive/10 text-destructive line-through' : ''}
                      "
                    >
                      <button
                        type="button"
                        class="
                          shrink-0 rounded-sm p-0.5 text-muted-foreground
                          hover:text-foreground
                        "
                        aria-label="Include {collection.name}"
                        title="Include {collection.name}"
                        onclick={() => onquery(addCollectionToQuery(tagQuery, collection.slug))}
                      >
                        <PlusIcon class="size-3" />
                      </button>
                      <button
                        type="button"
                        class="
                          shrink-0 rounded-sm p-0.5 text-muted-foreground
                          hover:text-foreground
                        "
                        aria-label="Exclude {collection.name}"
                        title="Exclude {collection.name}"
                        onclick={() => onquery(excludeCollectionFromQuery(tagQuery, collection.slug))}
                      >
                        <MinusIcon class="size-3" />
                      </button>
                      <!--
                        Clicking an active collection takes it out again
                        (spec "Filter from the list").
                      -->
                      <button
                        type="button"
                        class="min-w-0 flex-1 truncate py-1 text-left"
                        onclick={() => onquery(toggleCollectionInQuery(tagQuery, collection.slug))}
                      >
                        {collection.name}
                      </button>
                      <span class="shrink-0 text-muted-foreground tabular-nums">
                        {countById.get(collection.id) ?? ''}
                      </span>
                    </div>
                  {/snippet}
                </ContextMenu.Trigger>
                <ContextMenu.Content>
                  <ContextMenu.Item onSelect={() => (renaming = collection)}>
                    Rename…
                  </ContextMenu.Item>
                  <ContextMenu.Item variant="destructive" onSelect={() => (deleting = collection)}>
                    Delete…
                  </ContextMenu.Item>
                </ContextMenu.Content>
              </ContextMenu.Root>
            </li>
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
