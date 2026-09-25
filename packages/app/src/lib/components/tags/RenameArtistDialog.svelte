<script lang="ts">
  // `artist-entries` design D5, D6: opened from the inspector's tag menu on an
  // artist tag, and only there — the sidebar row and the pinned chip have no
  // image in scope to read a profile URL from. Mounted beside the menu, not
  // inside it: bits-ui destroys a menu's content the moment the item that
  // opened this closes it (`Inspector.svelte`'s `CollectionNameDialog` mount
  // is the same pattern).
  //
  // Stays mounted and follows `open` (the `CollectionNameDialog` shape),
  // rather than `{#if renamingArtist}`: a bits-ui `Dialog` torn down while
  // still open keeps running its own close effects against derived state that
  // no longer exists, which is what logged `derived_inert` on every Cancel
  // and submit.
  import type { ImageRecord, RenameArtistReport } from '@boorubox/shared'
  import { errorText, renameArtist, renameArtistPreview } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Textarea } from '$lib/components/ui/textarea'
  import { lines } from '$lib/components/artists/lines'

  interface Props {
    open: boolean
    /** The artist tag being renamed, as it reads on the image right now. */
    from: string
    /** The image whose record `renameArtistPreview` reads its candidate URL from. */
    image: ImageRecord | null
    /**
     * Where this dialog portals (design D3 of `browse-fixes`): the caller's
     * own `portalTarget` result, so opening it from inside the viewer lands it
     * in its `<dialog>` rather than underneath it — without this the dialog
     * opens in `<body>`, under the viewer's own top layer, and cannot be seen
     * or clicked there (the `CollectionNameDialog`/`UploadDialog` precedent).
     */
    portalTo?: Element
    /** Dismissed — by Cancel, the overlay or `Esc`. Nothing is written. */
    onclose: () => void
    /** `rename_artist` landed: the caller refreshes the vocabulary and the search. */
    onrenamed: (report: RenameArtistReport) => void
  }

  let { open, from, image, portalTo, onclose, onrenamed }: Props = $props()

  let nameInput = $state<HTMLInputElement | null>(null)
  let name = $state('')
  let urlsText = $state('')
  /** `null` until the preview answers — what disables the confirm button. */
  let carriers = $state<number | null>(null)
  let error = $state<string | null>(null)
  let saving = $state(false)

  // Follows whichever image the caller opens the dialog for (the
  // `CollectionNameDialog` pattern): `image` is `renamingArtist`'s own
  // snapshot in `Inspector.svelte`, not the live inspector image, so it only
  // changes on a fresh `open` transition, never mid-rename.
  $effect(() => {
    if (open && image) {
      name = from
      urlsText = ''
      carriers = null
      error = null
      saving = false
      const adapter = image.adapter
      void (async () => {
        try {
          const preview = await renameArtistPreview({ from, adapter })
          carriers = preview.carriers
          urlsText = preview.urls.join('\n')
        } catch (cause) {
          error = errorText(cause)
        }
      })()
    }
  })

  /** Nothing to do: Rust also short-circuits a rename onto the same name. */
  const unchanged = $derived(name.trim() === from)

  /**
   * Rust refuses an empty name, a name a non-artist category already owns, a
   * URL that does not normalise and a URL another artist owns, each naming
   * the offender (design D5), and that refusal is what is shown: a copy of
   * those checks here would be a second definition of what a valid rename is.
   */
  async function save() {
    if (saving || carriers === null || unchanged || !image) return
    saving = true
    error = null
    try {
      const report = await renameArtist({ from, to: name, urls: lines(urlsText) })
      onrenamed(report)
    } catch (cause) {
      error = errorText(cause)
    } finally {
      saving = false
    }
  }
</script>

<Dialog.Root {open} onOpenChange={(next) => { if (!next) onclose() }}>
  <Dialog.Content
    portalProps={{ to: portalTo }}
    class="sm:max-w-sm"
    onOpenAutoFocus={(event) => {
      // Selected, not only focused: overtyping the wrong name is the point of
      // this dialog, and a caret left at one end would make that a select-all
      // first (design D6). Done here, not in `onMount` (the `UploadDialog`
      // precedent for `onOpenAutoFocus`): bits-ui's own autofocus runs after
      // mount and would otherwise land the caret before this call does.
      event.preventDefault()
      nameInput?.select()
    }}
  >
    <Dialog.Header>
      <Dialog.Title>Rename artist</Dialog.Title>
      <Dialog.Description>
        Records the new name as the owner of these URLs and retags every image that carries
        “{from}”. A future capture from these URLs is tagged the new name too.
      </Dialog.Description>
    </Dialog.Header>

    <form
      class="flex flex-col gap-3"
      onsubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <div class="grid gap-1.5">
        <Label for="rename-artist-name" class="text-xs text-muted-foreground">New name</Label>
        <Input
          id="rename-artist-name"
          bind:ref={nameInput}
          bind:value={name}
          autocomplete="off"
        />
      </div>

      <div class="grid gap-1.5">
        <Label for="rename-artist-urls" class="text-xs text-muted-foreground">
          Profile URLs
          <span class="font-normal">— one per line</span>
        </Label>
        <Textarea
          id="rename-artist-urls"
          bind:value={urlsText}
          class="min-h-16 font-mono text-xs"
          spellcheck={false}
        />
      </div>

      {#if carriers === null && !error}
        <!-- Only while there is no error: a preview refusal is the line to show instead. -->
        <p class="text-xs text-muted-foreground">Reading how many images carry “{from}”…</p>
      {:else if carriers !== null}
        <p class="text-xs text-muted-foreground">
          {carriers.toLocaleString()} {carriers === 1 ? 'image carries' : 'images carry'} “{from}”
          and will be retagged “{name}”. Future captures from these URLs are tagged “{name}”.
        </p>
      {/if}

      {#if error}
        <p class="text-xs text-destructive">{error}</p>
      {/if}

      <Dialog.Footer>
        <Button type="button" variant="ghost" onclick={onclose}>Cancel</Button>
        <Button type="submit" disabled={saving || carriers === null || unchanged}>
          {carriers === null ? 'Rename' : `Rename ${carriers.toLocaleString()} ${carriers === 1 ? 'image' : 'images'}`}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
