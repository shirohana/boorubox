<script lang="ts">
  // Two inputs, not one (design D14): a bare word in the tag box is a tag, which
  // is the legacy syntax, so free text over page title and URLs needs its own
  // box. In the toolbar band there is no room for labels above them, so the
  // placeholders carry the distinction and `aria-label` carries it to a screen
  // reader.
  //
  // There is no Search button: the query runs after a typing pause and on
  // Enter, so a button would only ever repeat a search that had already run.
  import type { SearchInputs } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { KEY_ESCAPE } from '$lib/keyboard'

  let { onsearch }: { onsearch: (inputs: SearchInputs) => void } = $props()

  const TYPING_PAUSE_MS = 250

  let tagQuery = $state('')
  let text = $state('')
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

<form
  class="flex min-w-0 flex-1 items-center gap-2"
  onsubmit={(event) => {
    event.preventDefault()
    searchNow()
  }}
>
  <Input
    id="tag-query"
    class="h-8 min-w-0 flex-1"
    aria-label="Tags"
    bind:value={tagQuery}
    oninput={searchAfterPause}
    onkeydown={leaveOnEscape}
    placeholder="Tags — cat -dog · cat or dog · rating:s,q"
    autocomplete="off"
    spellcheck={false}
  />

  <Input
    id="text-query"
    class="h-8 min-w-0 flex-1"
    aria-label="Page title or URL"
    bind:value={text}
    oninput={searchAfterPause}
    onkeydown={leaveOnEscape}
    placeholder="Page title or URL"
    autocomplete="off"
    autocorrect="off"
    spellcheck={false}
  />

  {#if tagQuery || text}
    <Button type="button" size="sm" variant="ghost" onclick={clear}>Clear</Button>
  {/if}
</form>
