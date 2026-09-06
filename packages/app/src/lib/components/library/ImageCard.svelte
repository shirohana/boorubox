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
    /** The grid's current card: it holds the tab stop and shows its overlay. */
    focused: boolean
    onfocus: () => void
    onactivate: () => void
    onforget: () => void
  }

  let { image, focused, onfocus, onactivate, onforget }: Props = $props()

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

<!--
  Roving tabindex: exactly one card in the grid carries `tabindex="0"`, so Tab
  lands on the current card and the grid finds the element to focus by querying
  for it (`data-card-focus`). `onfocusin` rather than `onfocus` on the tile,
  because on a missing-file card the tab stop is its own button.
-->
<div class="aspect-square" onfocusin={onfocus}>
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
        flex h-full flex-col justify-between gap-2 overflow-hidden rounded-lg border border-dashed
        border-destructive/40 bg-destructive/5 p-3
        {focused ? 'ring-3 ring-ring/50' : ''}
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
        <Button
          size="xs"
          variant="outline"
          data-card-focus
          tabindex={focused ? 0 : -1}
          onclick={() => (confirmingForget = true)}
        >
          Remove record…
        </Button>
      {/if}
    </div>
  {:else}
    <button
      type="button"
      data-card-focus
      tabindex={focused ? 0 : -1}
      onclick={(event) => {
        // WebKit does not focus a button on click, and the grid's keys only
        // fire while the focus is inside it — so a clicked card has to take
        // the focus itself, or a click and then an arrow key does nothing.
        event.currentTarget.focus()
        onfocus()
      }}
      ondblclick={onactivate}
      class="
        group relative block size-full overflow-hidden rounded-lg border bg-muted/40 outline-none
        focus-visible:ring-3 focus-visible:ring-ring/50
        {focused ? 'border-ring' : 'border-border'}
      "
    >
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
        <span
          class="
            flex size-full items-center justify-center px-2 text-center text-xs
            text-muted-foreground
          "
        >
          {previewFailed ? 'No preview' : ''}
        </span>
      {/if}

      <!--
        The tile is the image (`library-browse`): its facts only ever cover it
        while it is hovered or focused, never as a caption strip below it.
      -->
      <span
        class="
          absolute inset-x-0 bottom-0 block bg-linear-to-t from-black/80 to-transparent px-2 pt-6
          pb-1.5 text-left opacity-0 transition-opacity
          group-hover:opacity-100
          group-focus-visible:opacity-100
          {focused ? 'opacity-100' : ''}
        "
      >
        <span class="block truncate text-xs text-white">{title}</span>
        <span class="block text-[0.7rem] text-white/70">{capturedOn} · {image.source}</span>
      </span>
    </button>
  {/if}
</div>
