<script lang="ts">
  // The one confirmation in the app: permanent deletion of images (`trash`
  // design D7), deleting a rule, removing a booru site, and moving two or more
  // images to the trash (design D12, amended). They ask the same question in
  // the same shape — a dialog per caller would be that sentence four times.
  // The bar for adding a fifth is still high: a confirmation on an act the user
  // can undo in one click teaches them to dismiss confirmations, and the trash
  // one clears it only on its scale — the count is what a `Cmd A` away from the
  // whole library does not otherwise say.
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'

  interface Props {
    /** The question, naming what is destroyed. */
    title: string
    /** What confirming does, and — where it matters — what it does not. */
    description: string
    /** The confirming button's words. */
    confirmLabel: string
    /**
     * Red button or the plain one. Default on, because three of the four
     * questions destroy; the trash move is reversible and its copy says so,
     * and a red button under that copy would say otherwise.
     */
    destructive?: boolean
    open: boolean
    /** Dismissed — by the button, the overlay or `Esc`. Nothing is destroyed. */
    onclose: () => void
    onconfirm: () => void
  }

  let {
    title, description, confirmLabel, destructive = true, open, onclose, onconfirm,
  }: Props = $props()
</script>

<!--
  The caller owns what is being destroyed, so it owns whether the dialog is up:
  a bindable `open` here would be a second copy of that, and the two would part
  the first time a dismissal had to leave the subject behind.

  `Dialog`, not `AlertDialog`: dismissing by the overlay or `Esc` cancels, which
  is the safe direction for a prompt whose other button cannot be undone.
-->
<Dialog.Root {open} onOpenChange={(next) => { if (!next) onclose() }}>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title>{title}</Dialog.Title>
      <Dialog.Description>{description}</Dialog.Description>
    </Dialog.Header>

    <Dialog.Footer>
      <Button variant="ghost" onclick={onclose}>Cancel</Button>
      <Button variant={destructive ? 'destructive' : 'default'} onclick={onconfirm}>{confirmLabel}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
