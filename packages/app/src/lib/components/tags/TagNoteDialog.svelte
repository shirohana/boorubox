<script lang="ts">
  // A tag's note, edited from its context menu (`tag-notes` design D10, D11):
  // mounted unconditionally by each host (`TagSidebar.svelte`, one;
  // `Inspector.svelte`, one for the chips of both strips and the badges), the
  // `CollectionNameDialog`/`RenameArtistDialog` shape — a bits-ui `Dialog`
  // torn down while still open logs `derived_inert` against derived state
  // that no longer exists. Reads the store directly rather than taking the
  // note as a prop: unlike a collection's name or an artist's URLs, there is
  // nowhere else the current text comes from.
  import { untrack } from 'svelte'
  import { vocabulary } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Textarea } from '$lib/components/ui/textarea'
  import { CATEGORY_TEXT_CLASS } from './categories'

  interface Props {
    open: boolean
    /** The tag name, snapshotted by the host when "Edit note…" is chosen. */
    name: string
    /**
     * Where this dialog portals (design D3 of `browse-fixes`): the caller's
     * own `portalTarget` result, so one opened from inside the viewer lands
     * in its `<dialog>` rather than underneath it. The sidebar's own mount
     * passes nothing — it is never inside the viewer.
     */
    portalTo?: Element
    /**
     * Dismissed — by Cancel, the overlay, `Esc`, or a no-op Save — or after a
     * Save lands. Nothing else is written.
     */
    onclose: () => void
  }

  let { open, name, portalTo, onclose }: Props = $props()

  let field = $state<HTMLTextAreaElement | null>(null)
  let text = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)

  // Follows whichever tag the caller opens the dialog for (the
  // `CollectionNameDialog` pattern): `vocabulary.noteOf` tracks `entries`,
  // reassigned wholesale by every `vocabulary.refresh()`, so read untracked —
  // otherwise a refresh while the dialog sits open reruns this effect and
  // resets `text` to the stored note, discarding what the user typed.
  $effect(() => {
    if (open) {
      text = untrack(() => vocabulary.noteOf(name)) ?? ''
      error = null
    }
  })

  /**
   * The same normalisation `set_note` stores (`tag-notes` design D2's
   * trim-and-blank-is-none), so "unchanged" compares what would actually
   * land rather than the raw typed text against the stored one.
   */
  const unchanged = $derived((text.trim() || null) === vocabulary.noteOf(name))

  async function save() {
    if (saving) return
    if (unchanged) {
      onclose()
      return
    }
    saving = true
    error = null
    const landed = await vocabulary.setNote(name, text)
    saving = false
    if (landed) {
      onclose()
    } else {
      error = vocabulary.error
    }
  }
</script>

<Dialog.Root {open} onOpenChange={(next) => { if (!next) onclose() }}>
  <Dialog.Content
    portalProps={{ to: portalTo }}
    class="sm:max-w-sm"
    onOpenAutoFocus={(event) => {
      // The caret at the end (`tag-notes` design D11), not bits-ui's own
      // autofocus, which would otherwise land it at the start.
      event.preventDefault()
      field?.focus()
      field?.setSelectionRange(text.length, text.length)
    }}
  >
    <Dialog.Header>
      <Dialog.Title>Tag note</Dialog.Title>
      <Dialog.Description class={CATEGORY_TEXT_CLASS[vocabulary.categoryOf(name)]}>
        {name}
      </Dialog.Description>
    </Dialog.Header>

    <form
      class="flex flex-col gap-3"
      onsubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <!--
        Enter here is a newline, not a confirmation (owner's rule: no new key
        binding for this) — so unlike `TagInput` and the facts form, no
        `onkeydown` catches it.
      -->
      <Textarea
        bind:ref={field}
        bind:value={text}
        class="min-h-24"
        aria-label="Tag note"
        placeholder="A short note about this tag"
      />

      {#if error}
        <p class="text-xs text-destructive">{error}</p>
      {/if}

      <Dialog.Footer>
        <Button type="button" variant="ghost" onclick={onclose}>Cancel</Button>
        <Button type="submit" disabled={saving}>Save</Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
