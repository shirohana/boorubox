<script lang="ts">
  // The tag field alone (`browse-feedback` design D3): the free-text field and
  // the 250 ms typing pause moved to `LibraryScreen`, which now owns both
  // fields' timer since they sit in two different regions of the frame — the
  // sidebar and the toolbar. What is left here is wiring only, same as before:
  // the rules live in `$lib/domain/tag-input`, and the syntax examples that
  // used to hide behind a `title` (the sidebar was too narrow for them beside
  // a second field) fit in the placeholder again now that this is the only
  // field in the section.
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { blurOnEscape } from '$lib/keyboard'

  interface Props {
    /**
     * Bound to the route (design D14 origin): the rating pills, the
     * collections and the inspector rewrite the query too, and this field has
     * to show what ran.
     */
    tagQuery: string
    /** The text changed by a keystroke or an accepted suggestion. */
    oninput: () => void
    /** Enter, with no suggestion highlighted: run the query now. */
    onsubmit: () => void
  }

  let { tagQuery = $bindable(), oninput, onsubmit }: Props = $props()
</script>

<section class="p-2">
  <h2 class="px-1 pb-1 text-xs font-medium text-muted-foreground">Search</h2>

  <TagInput
    id="tag-query"
    class="h-7 text-xs"
    label="Tags"
    bind:value={tagQuery}
    {oninput}
    {onsubmit}
    onescape={blurOnEscape}
    placeholder="cat -dog · cat or dog · rating:s,q"
  />
</section>
