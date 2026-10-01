<script lang="ts">
  // The dialog door of the pinned tags panel (`pinned-tags-panel` design D2):
  // a shell around `PinnedTagsPanel`. Mounted unconditionally by each host
  // beside its menu (`TagNoteDialog`'s reason: a Dialog torn down while open
  // logs `derived_inert`). The panel mounts with the dialog's content, so
  // every open starts with no ticks and no open fields.
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import PinnedTagsPanel from './PinnedTagsPanel.svelte'

  interface Props {
    open: boolean
    /** Where this dialog portals: the viewer's `<dialog>` when opened from inside it. */
    portalTo?: Element
    /** Dismissed. Every change has already been written. */
    onclose: () => void
  }

  let { open, portalTo, onclose }: Props = $props()
</script>

<Dialog.Root {open} onOpenChange={(next) => { if (!next) onclose() }}>
  <Dialog.Content portalProps={{ to: portalTo }} class="max-h-[85vh] grid-rows-[auto_minmax(0,1fr)] sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title>Pinned tags</Dialog.Title>
      <Dialog.Description>
        Pin and unpin tags, drag them between groups, name the groups and put them in order.
      </Dialog.Description>
    </Dialog.Header>

    <PinnedTagsPanel {portalTo}>
      {#snippet barEnd()}
        <Button type="button" onclick={onclose}>Done</Button>
      {/snippet}
    </PinnedTagsPanel>
  </Dialog.Content>
</Dialog.Root>
