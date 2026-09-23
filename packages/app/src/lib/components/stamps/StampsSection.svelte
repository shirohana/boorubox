<script lang="ts">
  // Slot Settings · Stamps (`stamps` design D5): the form and the table,
  // `RulesSection`'s own shape without the import/export/run row — a stamp
  // has nothing to run over existing images, it is applied by a click in
  // edit mode. The list itself is `api/stamps.svelte.ts`'s store, refreshed
  // by `Sidebar.svelte` on every library switch beside collections and the
  // vocabulary (design D3), so this section reads it rather than asking
  // again the way `RulesSection` asks for a list of its own.
  import type { Stamp } from '@boorubox/shared'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import { stamps as stampsStore } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import StampForm from './StampForm.svelte'
  import StampsTable from './StampsTable.svelte'

  /** Whether the form is open, and on which stamp — `null` is a new one. */
  let formOpen = $state(false)
  let editing = $state<Stamp | null>(null)
  let error = $state<string | null>(null)
</script>

<section class="flex flex-col gap-4">
  <h2 class="text-sm font-semibold">Stamps</h2>

  <p class="text-sm text-muted-foreground">
    A stamp is a saved edit, written once in the tag language (`cat animal -dog`,
    `collection:cute -collection:uncategorized`, `rating:g`) and applied to an image by a click
    in edit mode, or to a selection at once. Stamps are stored with the library, so copying the
    folder carries them.
  </p>

  {#if formOpen}
    <StampForm
      stamp={editing}
      onsaved={() => {
        formOpen = false
        editing = null
        error = null
        void stampsStore.refresh()
      }}
      oncancel={() => {
        formOpen = false
        editing = null
      }}
    />
  {:else}
    <div>
      <Button size="sm" onclick={() => (formOpen = true)}>
        <PlusIcon />
        New stamp
      </Button>
    </div>
  {/if}

  <StampsTable
    stamps={stampsStore.list}
    onedit={(stamp) => {
      editing = stamp
      formOpen = true
    }}
    onchanged={() => {
      // The re-read is what clears the section's last error (the table's
      // contract, copied from the rules'): the store's own error is not this one.
      error = null
      return stampsStore.refresh()
    }}
    onerror={(message) => (error = message)}
  />

  {#if error}
    <p class="text-sm text-destructive">{error}</p>
  {/if}
  {#if stampsStore.error}
    <p class="text-sm text-destructive">{stampsStore.error}</p>
  {/if}
</section>
