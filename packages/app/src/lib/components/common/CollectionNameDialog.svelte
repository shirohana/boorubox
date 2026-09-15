<script lang="ts">
  // `CollectionsSection`'s New collection… and Rename… (`collections` design
  // D8): one field, because a collection has one editable fact besides its
  // membership. Shared between create and rename — Rust refuses both the same
  // way, a blank name or a slug clash naming the collection that already
  // holds it (design D2, D3) — so a second form would be this one field twice.
  import type { Collection } from '@boorubox/shared'
  import { collectionCreate, collectionRename, errorText } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Input } from '$lib/components/ui/input'

  interface Props {
    /**
     * The collection being renamed, or `null` to create one. Only `id` and
     * `name` are read, so a `CollectionCount` row works here too — the
     * sidebar section never has to build a full `Collection` just to open
     * this dialog.
     */
    collection: Pick<Collection, 'id' | 'name'> | null
    open: boolean
    /** Dismissed — by the button, the overlay or `Esc`. Nothing is written. */
    onclose: () => void
    /** The collection as Rust stored it — the caller re-reads its list from this. */
    onsaved: (collection: Collection) => void
  }

  let { collection, open, onclose, onsaved }: Props = $props()

  let name = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)

  // Follows whichever collection the caller opens the dialog for (or none, to
  // create), so a rename opened on a second row while the dialog is still up
  // for the first shows that row's name rather than the one before it.
  $effect(() => {
    if (open) {
      name = collection?.name ?? ''
      error = null
    }
  })

  /**
   * Rust refuses a blank name and a slug clash with the reason (design D2,
   * D3), and that refusal is what is shown: a copy of those checks here would
   * be a second definition of what a valid collection name is.
   */
  async function save() {
    if (saving) return
    saving = true
    error = null
    try {
      const saved = collection
        ? await collectionRename(collection.id, name)
        : await collectionCreate(name)
      onsaved(saved)
    } catch (cause) {
      // The field keeps what was typed: nothing was saved, and retyping it is
      // the last thing anyone wants after being told why.
      error = errorText(cause)
    } finally {
      saving = false
    }
  }
</script>

<Dialog.Root {open} onOpenChange={(next) => { if (!next) onclose() }}>
  <Dialog.Content class="sm:max-w-sm">
    <Dialog.Header>
      <Dialog.Title>{collection ? `Rename “${collection.name}”` : 'New collection'}</Dialog.Title>
      <Dialog.Description>
        {collection
          ? 'The images in it are unchanged; only its name and search term change.'
          : 'A named set of images you add to from the tile menu, the toolbar or the inspector.'}
      </Dialog.Description>
    </Dialog.Header>

    <form
      class="flex flex-col gap-3"
      onsubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <Input
        bind:value={name}
        autocomplete="off"
        autofocus
        placeholder="Collection name"
        aria-label="Collection name"
      />

      {#if error}
        <p class="text-xs text-destructive">{error}</p>
      {/if}

      <Dialog.Footer>
        <Button type="button" variant="ghost" onclick={onclose}>Cancel</Button>
        <Button type="submit" disabled={saving}>{collection ? 'Rename' : 'Create'}</Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
