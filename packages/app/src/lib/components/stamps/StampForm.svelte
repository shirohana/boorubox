<script lang="ts">
  // Slot Settings · Stamps, the editing half (`stamps` design D5): name and
  // text, `RuleForm`'s own shape without the pattern and the regex switch — a
  // stamp is checked by `parseStamp` in the webview, never matched against an
  // image by Rust. One form for both creating and editing, and for the bar's
  // "Save as stamp…" dialog (design D5), which is why the one-off's typed
  // text arrives as `initialText` rather than only ever from `stamp`.
  import type { Stamp } from '@boorubox/shared'
  import { errorText, stampsUpsert } from '$lib/api'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { parseStamp } from '$lib/domain/stamp'

  interface Props {
    /** The stamp being edited, or `null` to create one. */
    stamp: Stamp | null
    /** The bar's one-off text, prefilled when "Save as stamp…" opens this (design D5). */
    initialText?: string
    /** The rule as Rust stored it — the caller re-reads the list from this. */
    onsaved: (stamp: Stamp) => void
    oncancel: () => void
  }

  let { stamp, initialText = '', onsaved, oncancel }: Props = $props()

  let name = $state('')
  let text = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)

  // The fields follow whichever stamp the caller hands over, so clicking Edit
  // on a second row while the first is open loads that row rather than
  // leaving the form showing the one before it — `RuleForm`'s own effect.
  $effect(() => {
    name = stamp?.name ?? ''
    text = stamp?.text ?? initialText
    error = null
  })

  /**
   * `parseStamp`'s reason, shown as it is typed (design D5): a text that
   * cannot be an edit is never worth a round trip to Rust to learn that —
   * Rust only refuses an empty name or text, never an invalid grammar.
   */
  const parsed = $derived(parseStamp(text))
  const liveError = $derived('error' in parsed ? parsed.error : null)

  async function save() {
    if (saving || liveError) return
    saving = true
    error = null
    try {
      onsaved(await stampsUpsert({ id: stamp?.id, name, text }))
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
    <Label for="stamp-name" class="text-xs text-muted-foreground">Name</Label>
    <Input id="stamp-name" class="h-8" bind:value={name} autocomplete="off" />
  </div>

  <div class="grid gap-1.5">
    <Label for="stamp-text" class="text-xs text-muted-foreground">
      Text
      <span class="font-normal">
        — tags, `-tag`, `collection:x`, `-collection:x`, `rating:g|s|q|e`
      </span>
    </Label>
    <TagInput
      id="stamp-text"
      bind:value={text}
      label="What this stamp writes"
      placeholder="cat animal -dog"
      class="h-8"
    />
  </div>

  {#if liveError}
    <p class="text-xs text-destructive">{liveError}</p>
  {:else if error}
    <p class="text-xs text-destructive">{error}</p>
  {/if}

  <div class="flex justify-end gap-2">
    <Button type="button" size="sm" variant="ghost" onclick={oncancel}>Cancel</Button>
    <Button type="submit" size="sm" disabled={saving || Boolean(liveError)}>
      {stamp ? 'Save stamp' : 'Add stamp'}
    </Button>
  </div>
</form>
