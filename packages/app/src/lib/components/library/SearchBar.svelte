<script lang="ts">
  // Two inputs, not one (design D14): a bare word in the tag box is a tag, which
  // is the legacy syntax, so free text over page title and URLs needs its own
  // box. The labels are what tell the user which is which.
  import type { SearchInputs } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'

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

  $effect(() => () => clearTimeout(timer))
</script>

<form
  class="flex flex-wrap items-end gap-3"
  onsubmit={(event) => {
    event.preventDefault()
    searchNow()
  }}
>
  <div class="min-w-56 flex-1">
    <label class="mb-1 block text-xs font-medium text-muted-foreground" for="tag-query">
      Tags
    </label>
    <Input
      id="tag-query"
      bind:value={tagQuery}
      oninput={searchAfterPause}
      placeholder="cat -dog · cat or dog · rating:s,q"
      autocomplete="off"
      spellcheck={false}
    />
  </div>

  <div class="min-w-56 flex-1">
    <label class="mb-1 block text-xs font-medium text-muted-foreground" for="text-query">
      Page title or URL
    </label>
    <Input
      id="text-query"
      bind:value={text}
      oninput={searchAfterPause}
      placeholder="free text"
      autocomplete="off"
    />
  </div>

  <Button type="submit" variant="secondary">Search</Button>
  {#if tagQuery || text}
    <Button type="button" variant="ghost" onclick={clear}>Clear</Button>
  {/if}
</form>
