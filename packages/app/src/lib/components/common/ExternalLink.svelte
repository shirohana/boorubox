<script lang="ts">
  // The open-in-browser action beside an address in the inspector (design D5),
  // both placements. `PostedLabel` opens a post the same way — through
  // `openExternal` — but keeps its own text-link shape; this is the icon-only
  // button the Page and Image rows use.
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link'
  import { openExternal } from '$lib/api'
  import { Button } from '$lib/components/ui/button'

  let { url }: { url: string | null } = $props()

  /** A `file:` address or a malformed one has no browser to go to (design D5). */
  const openable = $derived.by(() => {
    if (!url) return false
    try {
      return ['http:', 'https:'].includes(new URL(url).protocol)
    } catch {
      return false
    }
  })

  /** Why the last attempt failed; silence would look like a dead button. */
  let failure = $state<string | null>(null)

  async function open() {
    if (!url) return
    failure = await openExternal(url)
  }
</script>

{#if openable && url}
  <!--
    `contents`, so the failure line is a child of the row rather than of a
    column beside the address: as one flex item holding both, the wrapper's
    width became the message's, and the address next to it — `flex-1 min-w-0`,
    which shrinks to nothing before anything else does — collapsed to zero.
    The row wraps, the message takes a line of its own under the button.
  -->
  <span class="contents">
    <Button
      type="button"
      size="icon-xs"
      variant="ghost"
      aria-label="Open in browser"
      title="Open in browser"
      onclick={open}
    >
      <ExternalLinkIcon class="size-3" />
    </Button>
    {#if failure}
      <p class="mt-1 w-full text-xs text-destructive">This link could not be opened: {failure}</p>
    {/if}
  </span>
{/if}
