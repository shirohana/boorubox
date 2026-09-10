<script lang="ts">
  // Where this image has already been posted (`posted-label`), one entry per
  // site. Shown in the Inspector in both of its placements and absent — not
  // empty — for an image that has never been posted.
  import type { PostRef } from '@boorubox/shared'
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link'
  import { booruSites, openExternal } from '$lib/api'
  import { formatTimestamp } from '$lib/domain/format'
  import { postedEntries, postedName } from './posted'

  let { posts }: { posts: PostRef[] } = $props()

  const entries = $derived(postedEntries(posts, booruSites.sites))

  /** Why the last link did not open; silence would look like a dead button. */
  let linkFailure = $state<string | null>(null)

  async function openLink(url: string) {
    linkFailure = await openExternal(url)
  }
</script>

<ul class="flex flex-col gap-1">
  {#each entries as entry (entry.key)}
    <li class="text-xs">
      {#if entry.url}
        <button
          type="button"
          class="inline-flex items-center gap-1 underline-offset-2 hover:underline"
          title={entry.url}
          onclick={() => {
            if (entry.url) void openLink(entry.url)
          }}
        >
          <span>{postedName(entry.name, entry.remoteId)}</span>
          <ExternalLinkIcon class="size-3 shrink-0" />
        </button>
      {:else}
        <!--
          Design D2: the record outlives the site it was made against, so a
          removed site leaves the entry standing with the key it was written
          with — and no link, because nothing knows the address any more.
        -->
        <span title="This site is no longer configured">
          {postedName(entry.name, entry.remoteId)}
        </span>
      {/if}
      <span class="text-muted-foreground"> · {formatTimestamp(entry.postedAt)}</span>
    </li>
  {/each}
</ul>

{#if linkFailure}
  <p class="mt-1 text-xs text-destructive">This link could not be opened: {linkFailure}</p>
{/if}
