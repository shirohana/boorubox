<script lang="ts">
  // The spec asks for an empty state that names the query: a blank grid reads as
  // a broken app, and the user cannot see which of the two boxes ruled the
  // library out.
  import type { SearchInputs } from '$lib/api'

  interface Props {
    inputs: SearchInputs
    /** Which set found nothing (`trash` design D1): they read differently. */
    view: 'library' | 'trash'
  }

  let { inputs, view }: Props = $props()

  const searching = $derived(inputs.tagQuery.trim() !== '' || inputs.text.trim() !== '')
</script>

<div class="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
  {#if searching}
    <p class="text-sm font-medium">No images match this search.</p>
    <dl class="text-sm text-muted-foreground">
      {#if inputs.tagQuery.trim()}
        <div class="flex justify-center gap-2">
          <dt>Tags</dt>
          <dd class="font-mono break-all text-foreground">{inputs.tagQuery.trim()}</dd>
        </div>
      {/if}
      {#if inputs.text.trim()}
        <div class="flex justify-center gap-2">
          <dt>Page title or URL</dt>
          <dd class="font-mono break-all text-foreground">{inputs.text.trim()}</dd>
        </div>
      {/if}
    </dl>
  {:else if view === 'trash'}
    <p class="text-sm font-medium">The trash is empty.</p>
    <p class="text-sm text-muted-foreground">
      Images you delete are kept here until you restore them or delete them for good.
    </p>
  {:else}
    <p class="text-sm font-medium">This library has no images yet.</p>
    <p class="text-sm text-muted-foreground">
      Drop image files or folders on this window, or capture from the browser extension.
    </p>
  {/if}
</div>
