<script lang="ts">
  // Slot Sidebar · filters (design D17), the Collections section (`collections`
  // design D8): every collection with its count in the current result, active
  // first (spec "The sidebar lists the collections with counts"). Counts ride
  // on `results.counts.collections`, the same round trip the tag list already
  // gets (design D7) — nothing here counts a membership of its own.
  import type { CollectionCount } from '@boorubox/shared'
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import { collectionDelete, collections, errorText } from '$lib/api'
  import CollectionNameDialog from '$lib/components/common/CollectionNameDialog.svelte'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { activeTerms, toggleCollectionInQuery } from '$lib/domain/tag-utils'

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

  // Design D3: the one reader for "is this term active", shared with the tag
  // sidebar and the inspector's badges.
  const terms = $derived(activeTerms(tagQuery))

  /**
   * Rust already orders `counts` by name (design D7); the only thing computed
   * here is which half a row falls in — a stable sort keeps each half in that
   * order, so this only ever moves the search's own collections in front
   * (spec: "with the collections the search names first").
   */
  const listed = $derived.by(() => {
    if (!counts) return null
    const isActive = (slug: string) =>
      terms.collections.has(slug) || terms.excludedCollections.has(slug)
    return [...counts].sort((a, b) => Number(isActive(b.slug)) - Number(isActive(a.slug)))
  })

  let creating = $state(false)
  let renaming = $state<CollectionCount | null>(null)
  let deleting = $state<CollectionCount | null>(null)
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
    `${deleting?.count ?? 0} of the images this search matches are in it.`,
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

<section class="p-2">
  <div class="flex items-center justify-between px-1 pb-1">
    <h2 class="text-xs font-medium text-muted-foreground">Collections</h2>
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

  {#if listed && listed.length === 0}
    <p class="px-1 text-xs text-muted-foreground">No collections</p>
  {:else if listed}
    <ul class="flex flex-col gap-0.5">
      {#each listed as collection (collection.id)}
        {@const active = terms.collections.has(collection.slug)}
        {@const excluded = terms.excludedCollections.has(collection.slug)}
        <li
          class="
            flex items-center gap-1 rounded-md px-1 text-xs
            hover:bg-sidebar-accent
            {active ? 'bg-emerald-500/15 font-medium text-emerald-700 dark:text-emerald-300' : ''}
            {excluded ? 'bg-destructive/10 text-destructive line-through' : ''}
          "
        >
          <!-- Clicking an active collection takes it out again (spec "Filter from the list"). -->
          <button
            type="button"
            class="min-w-0 flex-1 truncate py-1 text-left"
            onclick={() => onquery(toggleCollectionInQuery(tagQuery, collection.slug))}
          >
            {collection.name}
          </button>
          <span class="shrink-0 text-muted-foreground tabular-nums">{collection.count}</span>

          <DropdownMenu.Root>
            <DropdownMenu.Trigger>
              {#snippet child({ props })}
                <button
                  type="button"
                  class="shrink-0 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
                  aria-label="{collection.name} menu"
                  {...props}
                >
                  <EllipsisIcon class="size-3" />
                </button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content align="end">
              <DropdownMenu.Item onSelect={() => (renaming = collection)}>
                Rename…
              </DropdownMenu.Item>
              <DropdownMenu.Item variant="destructive" onSelect={() => (deleting = collection)}>
                Delete…
              </DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Root>
        </li>
      {/each}
    </ul>
  {/if}

  {#if error}
    <p class="px-1 pt-1 text-xs text-destructive">{error}</p>
  {/if}
</section>

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
