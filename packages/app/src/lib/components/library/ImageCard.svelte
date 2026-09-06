<script module lang="ts">
  // One `thumbnail_path` round trip per image for the life of the window. The
  // grid recycles its cards while scrolling, so without this every pass over a
  // row would ask Rust for a path it already has.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a plain cache, never rendered from
  const thumbnails = new Map<string, string>()
</script>

<script lang="ts">
  import type { ImageRecord } from '@boorubox/shared'
  import { thumbnailUrl } from '$lib/api'
  import { Button } from '$lib/components/ui/button'

  interface Props {
    /** `undefined` while the page holding this row is still loading. */
    image: ImageRecord | undefined
    height: number
    onactivate: () => void
    onforget: () => void
  }

  let { image, height, onactivate, onforget }: Props = $props()

  let src = $state<string | null>(null)
  let previewFailed = $state(false)
  let confirmingForget = $state(false)

  const title = $derived(image?.pageTitle || image?.imageUrl || image?.id || '')
  const capturedOn = $derived(image ? new Date(image.capturedAt).toLocaleDateString() : '')

  $effect(() => {
    const id = image && !image.missing ? image.id : null
    previewFailed = false
    confirmingForget = false
    const cached = id ? thumbnails.get(id) ?? null : null
    src = cached
    if (!id || cached) return

    let current = true
    void thumbnailUrl(id)
      .then((url) => {
        thumbnails.set(id, url)
        if (current) src = url
      })
      .catch(() => {
        if (current) previewFailed = true
      })
    return () => {
      current = false
    }
  })
</script>

<div style="height: {height}px">
  {#if !image}
    <div class="h-full rounded-lg border border-border bg-muted/40">
      <span class="sr-only">Loading</span>
    </div>
  {:else if image.missing}
    <!--
      Design D16: dropping a record deletes the row and its thumbnail only. The
      wording must not suggest the image file is removed, because it is not.
    -->
    <div
      class="
        flex h-full flex-col justify-between gap-2 rounded-lg border border-dashed
        border-destructive/40 bg-destructive/5 p-3
      "
    >
      <div class="min-h-0">
        <p class="text-xs font-medium text-destructive">File not found</p>
        <p class="mt-1 line-clamp-3 text-xs break-all text-muted-foreground">{title}</p>
      </div>

      {#if confirmingForget}
        <div>
          <p class="text-xs text-muted-foreground">
            Remove this record from the library? Nothing on disk is deleted.
          </p>
          <div class="mt-2 flex gap-2">
            <Button
              size="xs"
              variant="destructive"
              onclick={() => {
                confirmingForget = false
                onforget()
              }}
            >
              Remove record
            </Button>
            <Button size="xs" variant="ghost" onclick={() => (confirmingForget = false)}>
              Cancel
            </Button>
          </div>
        </div>
      {:else}
        <Button size="xs" variant="outline" onclick={() => (confirmingForget = true)}>
          Remove record…
        </Button>
      {/if}
    </div>
  {:else}
    <button
      type="button"
      onclick={onactivate}
      class="
        flex size-full flex-col overflow-hidden rounded-lg border border-border bg-card text-left
        outline-none
        focus-visible:ring-3 focus-visible:ring-ring/50
      "
    >
      <span class="flex min-h-0 flex-1 items-center justify-center bg-muted/40">
        {#if src}
          <img
            {src}
            alt={title}
            loading="lazy"
            decoding="async"
            class="size-full object-contain"
            onerror={() => (previewFailed = true)}
          />
        {:else}
          <span class="px-2 text-center text-xs text-muted-foreground">
            {previewFailed ? 'No preview' : ''}
          </span>
        {/if}
      </span>
      <span class="block w-full px-2 py-1.5">
        <span class="block truncate text-xs text-foreground">{title}</span>
        <span class="block text-[0.7rem] text-muted-foreground">
          {capturedOn} · {image.source}
        </span>
      </span>
    </button>
  {/if}
</div>
