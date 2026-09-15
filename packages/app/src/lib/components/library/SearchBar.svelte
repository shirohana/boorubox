<script lang="ts">
  // Two inputs, not one (design D14): a bare word in the tag box is a tag, which
  // is the legacy syntax, so free text over page title and URLs needs its own
  // box. Stacked in the sidebar (`sidebar-layout` design D2) there is no room
  // for the example syntax in the placeholder either, so it moves to the tag
  // field's `title` and shows on hover instead; `aria-label` still carries the
  // text field's purpose to a screen reader.
  //
  // There is no Search button: the query runs after a typing pause and on
  // Enter, so a button would only ever repeat a search that had already run.
  import type { SearchInputs } from '$lib/api'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { KEY_ESCAPE } from '$lib/keyboard'

  interface Props {
    /**
     * Bound to the route, because the sidebar, the rating pills and the
     * inspector rewrite the query too (design D14) and this field has to show
     * what ran.
     */
    tagQuery: string
    text: string
    onsearch: (inputs: SearchInputs) => void
  }

  let { tagQuery = $bindable(), text = $bindable(), onsearch }: Props = $props()

  const TYPING_PAUSE_MS = 250

  let timer: ReturnType<typeof setTimeout> | undefined

  function searchNow() {
    clearTimeout(timer)
    onsearch({ tagQuery, text })
  }

  function searchAfterPause() {
    clearTimeout(timer)
    timer = setTimeout(searchNow, TYPING_PAUSE_MS)
  }

  function clear() {
    tagQuery = ''
    text = ''
    searchNow()
  }

  /** The keyboard map's way back out of a field, so the grid keys are live again. */
  function leaveOnEscape(event: KeyboardEvent) {
    if (event.key !== KEY_ESCAPE) return
    event.preventDefault()
    if (event.currentTarget instanceof HTMLElement) event.currentTarget.blur()
  }

  $effect(() => () => clearTimeout(timer))
</script>

<section class="p-2">
  <h2 class="px-1 pb-1 text-xs font-medium text-muted-foreground">Search</h2>

  <form
    class="flex flex-col gap-1"
    onsubmit={(event) => {
      event.preventDefault()
      searchNow()
    }}
  >
    <!--
      Slot Sidebar · search: the same editor the inspector uses (design D17).
      `multiline` (`sidebar-layout` design D2): the sidebar is narrower than
      the old toolbar band, so a long query wraps instead of scrolling
      sideways out of view. The wrapping `title` is what carries the example
      syntax now that the placeholder is too short to hold it — a title on an
      ancestor with none of its own is what the field's tooltip falls back to.
    -->
    <div title="cat -dog · cat or dog · rating:s,q">
      <TagInput
        id="tag-query"
        class="max-h-32 min-h-7 text-xs"
        label="Tags"
        multiline
        bind:value={tagQuery}
        oninput={searchAfterPause}
        onsubmit={searchNow}
        onescape={leaveOnEscape}
        placeholder="Tags"
      />
    </div>

    <Input
      id="text-query"
      class="h-7 text-xs"
      aria-label="Page title or URL"
      bind:value={text}
      oninput={searchAfterPause}
      onkeydown={leaveOnEscape}
      placeholder="Title or URL"
      autocomplete="off"
      autocorrect="off"
      spellcheck={false}
    />

    {#if tagQuery || text}
      <Button type="button" size="sm" variant="ghost" class="self-end" onclick={clear}>
        Clear
      </Button>
    {/if}
  </form>
</section>
