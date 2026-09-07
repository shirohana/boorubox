<script lang="ts">
  // No image (design D8): the bytes the announcement names have not been
  // downloaded yet, and loading `imageUrl` here would be a second fetch of the
  // same picture — a failing one on a host that checks the referer.
  import type { CaptureMeta } from '@boorubox/shared'
  import { Skeleton } from '$lib/components/ui/skeleton'

  let { capture }: { capture: CaptureMeta } = $props()

  const site = $derived(capture.adapter?.site ?? null)
  const title = $derived(capture.pageTitle || capture.pageUrl)
</script>

<div class="relative aspect-square">
  <Skeleton class="absolute inset-0 rounded-lg" />
  <div class="absolute inset-x-0 bottom-0 p-2">
    {#if site}
      <p class="truncate text-xs font-medium">{site}</p>
    {/if}
    <p class="line-clamp-2 text-xs break-all text-muted-foreground">{title}</p>
  </div>
</div>
