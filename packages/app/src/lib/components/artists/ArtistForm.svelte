<script lang="ts">
  // Slot Settings · Artists, the editing half (`artist-entries` design D7):
  // one form for both — creating and editing share the same refusals
  // (`artists_upsert` design D1, D5), so a second form would be the same two
  // fields with the tag input swapped for static text. The tag is read-only
  // once an entry exists: `artists_upsert` replaces a tag's whole URL set,
  // it never renames one — that is the inspector's "Rename artist…" (the
  // proposal's non-goal on a general tag rename).
  import type { ArtistEntry } from '@boorubox/shared'
  import { artistsUpsert, errorText } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Textarea } from '$lib/components/ui/textarea'
  import { lines } from './lines'

  interface Props {
    /** The entry being edited, or `null` to create one. */
    entry: ArtistEntry | null
    /** The entries as Rust stored them — the caller re-reads the list from this. */
    onsaved: (entries: ArtistEntry[]) => void
    oncancel: () => void
  }

  let { entry, onsaved, oncancel }: Props = $props()

  let tag = $state('')
  let urlsText = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)

  // Follows whichever entry the caller hands over, so clicking Edit on a
  // second row while the first is open loads that row rather than leaving
  // the form showing the one before it.
  $effect(() => {
    tag = entry?.tag ?? ''
    urlsText = entry?.urls.join('\n') ?? ''
    error = null
  })

  /**
   * Rust refuses an empty tag, a URL that does not normalise and a URL
   * another artist owns, each naming the offender (design D1, D5), and that
   * refusal is what is shown: a copy of those checks here would be a second
   * definition of what a valid entry is.
   */
  async function save() {
    if (saving) return
    saving = true
    error = null
    try {
      onsaved(await artistsUpsert({ tag, urls: lines(urlsText) }))
    } catch (cause) {
      // The fields keep what was typed: nothing was saved, and retyping it is
      // the last thing anyone wants after being told why.
      error = errorText(cause)
    } finally {
      saving = false
    }
  }
</script>

<form
  class="flex flex-col gap-3 rounded-lg border border-border p-3"
  onsubmit={(event) => {
    event.preventDefault()
    void save()
  }}
>
  <div class="grid gap-1.5">
    <Label for="artist-tag" class="text-xs text-muted-foreground">Tag</Label>
    <Input
      id="artist-tag"
      class="h-8"
      bind:value={tag}
      autocomplete="off"
      autofocus={!entry}
      readonly={!!entry}
    />
  </div>

  <div class="grid gap-1.5">
    <Label for="artist-urls" class="text-xs text-muted-foreground">
      Profile URLs
      <span class="font-normal">— one per line</span>
    </Label>
    <Textarea
      id="artist-urls"
      bind:value={urlsText}
      class="min-h-16 font-mono text-xs"
      spellcheck={false}
    />
  </div>

  {#if error}
    <p class="text-xs text-destructive">{error}</p>
  {/if}

  <div class="flex justify-end gap-2">
    <Button type="button" size="sm" variant="ghost" onclick={oncancel}>Cancel</Button>
    <Button type="submit" size="sm" disabled={saving}>
      {entry ? 'Save' : 'Add artist'}
    </Button>
  </div>
</form>
