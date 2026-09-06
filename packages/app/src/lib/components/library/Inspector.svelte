<script lang="ts">
  // One component, two placements (design D9): the library route's right column
  // and the lightbox's inspect mode. Read-only on purpose — `app-frame` says a
  // fact that cannot be changed yet is shown as text with no editing
  // affordance, so `tags-and-ratings` has one editor to build, not two.
  import type { ImageRecord } from '@boorubox/shared'
  import { Badge } from '$lib/components/ui/badge'
  import { formatBytes, formatTimestamp, ratingLabel } from '$lib/domain/format'
  import { sortTags } from '$lib/domain/tag-utils'

  interface Props {
    image: ImageRecord | null
  }

  let { image }: Props = $props()

  const title = $derived(image?.pageTitle || image?.imageUrl || image?.id || '')
  const origin = $derived(
    image ? (image.sourceRef ? `${image.source} · ${image.sourceRef}` : image.source) : '',
  )
  const tags = $derived(image ? sortTags(image.tags) : [])
</script>

<div class="flex h-full flex-col overflow-y-auto">
  {#if !image}
    <p class="p-4 text-sm text-muted-foreground">No image selected</p>
  {:else}
    <header class="border-b border-border px-4 py-3">
      <h2 class="text-sm font-medium wrap-break-word">{title}</h2>
    </header>

    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 px-4 py-3 text-xs">
      <!--
        The header falls back to the image URL and then the id so it is never
        blank; this row is the stored page title itself, which is often absent.
      -->
      <dt class="text-muted-foreground">Title</dt>
      <dd class="wrap-break-word">{image.pageTitle ?? '—'}</dd>

      <dt class="text-muted-foreground">Source</dt>
      <dd class="wrap-break-word">{origin}</dd>

      <dt class="text-muted-foreground">Page</dt>
      <dd class="wrap-break-word">{image.pageUrl ?? '—'}</dd>

      <dt class="text-muted-foreground">Image</dt>
      <dd class="wrap-break-word">{image.imageUrl ?? '—'}</dd>

      <dt class="text-muted-foreground">Dimensions</dt>
      <dd>{image.width} × {image.height}</dd>

      <dt class="text-muted-foreground">Size</dt>
      <dd>{formatBytes(image.size)}</dd>

      <dt class="text-muted-foreground">Type</dt>
      <dd>{image.mime}</dd>

      <dt class="text-muted-foreground">Captured</dt>
      <dd>{formatTimestamp(image.capturedAt)}</dd>

      <dt class="text-muted-foreground">Imported</dt>
      <dd>{formatTimestamp(image.createdAt)}</dd>

      <dt class="text-muted-foreground">ID</dt>
      <dd class="font-mono wrap-break-word">{image.id}</dd>
    </dl>

    <section class="border-t border-border px-4 py-3">
      <h3 class="mb-2 text-xs font-medium text-muted-foreground">Rating</h3>
      <p class="text-xs">{ratingLabel(image.rating)}</p>
    </section>

    <section class="border-t border-border px-4 py-3">
      <h3 class="mb-2 text-xs font-medium text-muted-foreground">
        Tags {#if tags.length > 0}({tags.length}){/if}
      </h3>
      {#if tags.length === 0}
        <p class="text-xs text-muted-foreground">No tags</p>
      {:else}
        <ul class="flex flex-wrap gap-1">
          {#each tags as tag (tag)}
            <li><Badge variant="secondary">{tag}</Badge></li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>
