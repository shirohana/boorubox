<script lang="ts">
  // The shared submenu body for every place a collection action lives
  // (`collections` design D8): the tile's context menu, the selection
  // toolbar's dropdown and the inspector's "Add to…" menu each mount this
  // with their own menu chrome — `ui/context-menu`'s submenu content or
  // `ui/dropdown-menu`'s content, bits-ui's two primitives, not
  // interchangeable at runtime — through the `checkboxItem`/`item`/`separator`/
  // `sub` snippets. The list and the checkmark (or, with no memberships to
  // check, the Add/Remove submenu) live here once; the write lives in
  // `collection-actions.ts`, and the "New collection…" dialog is the caller's
  // to mount *outside* its menu, because bits-ui unmounts a menu's content the
  // moment the item that opened the dialog closes it.
  import type { Snippet } from 'svelte'
  import { collections } from '$lib/api'
  import type { CollectionTarget } from './collection-actions'
  import { addToCollection, removeFromCollection, toggleCollection } from './collection-actions'

  interface Props {
    /** Which images this menu acts on, and what they are already in. */
    target: CollectionTarget
    /** "New collection…" was chosen: the caller raises its own dialog. */
    onnew: () => void
    /** One collection row, checked when every named image is in it. */
    checkboxItem: Snippet<[{ label: string, checked: boolean, onSelect: () => void }]>
    /** The "New collection…" row, in the caller's own plain item type. */
    item: Snippet<[{ label: string, onSelect: () => void }]>
    separator: Snippet
    /**
     * A collection row when `target.memberships()` is `null` (the toolbar): a
     * range selection can span rows the app never loaded, so there is no
     * membership to check a box against, and the row opens a submenu of
     * `item`s ("Add selection" / "Remove selection") instead. Optional: the
     * tile and the inspector always know their own memberships and never
     * render this branch, so they need not supply it.
     */
    sub?: Snippet<[{ label: string, children: Snippet }]>
  }

  let { target, onnew, checkboxItem, item, separator, sub }: Props = $props()
</script>

{#each collections.list as collection (collection.id)}
  {@const memberships = target.memberships()}
  {#if memberships === null && sub}
    {#snippet addRemoveItems()}
      {@render item({
        label: 'Add selection',
        onSelect: () => void addToCollection(target, collection.id),
      })}
      {@render item({
        label: 'Remove selection',
        onSelect: () => void removeFromCollection(target, collection.id),
      })}
    {/snippet}
    {@render sub({ label: collection.name, children: addRemoveItems })}
  {:else}
    {@render checkboxItem({
      label: collection.name,
      checked: memberships?.has(collection.id) ?? false,
      onSelect: () => void toggleCollection(target, collection.id),
    })}
  {/if}
{/each}

{@render separator()}
{@render item({ label: 'New collection…', onSelect: onnew })}
