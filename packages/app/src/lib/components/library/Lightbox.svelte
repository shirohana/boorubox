<script lang="ts">
  // A native modal dialog, for the focus behaviour rather than the looks: the
  // browser traps Tab inside it, closes it on Escape, and returns focus to the
  // thumbnail that opened it. Hand-rolling that is how keyboard users get
  // stranded.
  //
  // Arrow navigation is BOUNDED, not clamped: at either end nothing happens.
  // `offsetIndexBounded` is that rule; the buttons read it too, so the disabled
  // state and the keys can never disagree.
  import type { SearchResults } from '$lib/api'
  import { imageUrl } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import { offsetIndexBounded } from '$lib/domain/navigation-math'

  interface Props {
    results: SearchResults
    index: number
    libraryPath: string | null
    onclose: () => void
  }

  let { results, index = $bindable(), libraryPath, onclose }: Props = $props()

  let dialog = $state<HTMLDialogElement | null>(null)

  const image = $derived(results.at(index))
  const src = $derived(image && libraryPath ? imageUrl(libraryPath, image) : null)
  const title = $derived(image?.pageTitle || image?.imageUrl || image?.id || '')
  const previous = $derived(offsetIndexBounded(index, -1, results.total))
  const next = $derived(offsetIndexBounded(index, 1, results.total))

  $effect(() => {
    dialog?.showModal()
  })

  function move(destination: number | null) {
    if (destination === null) return
    index = destination
    results.ensureRange(destination, destination + 1)
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft') {
      event.preventDefault()
      move(previous)
    } else if (event.key === 'ArrowRight') {
      event.preventDefault()
      move(next)
    }
  }
</script>

<dialog
  bind:this={dialog}
  {onclose}
  {onkeydown}
  class="
    m-auto h-[92vh] max-h-none w-[92vw] max-w-none rounded-xl border border-border bg-background p-0
    text-foreground
    backdrop:bg-black/70
  "
>
  <div class="flex h-full flex-col">
    <header class="flex items-center justify-between gap-3 border-b border-border px-3 py-2">
      <div class="min-w-0">
        <p class="truncate text-sm">{title}</p>
        <p class="text-xs text-muted-foreground">
          {(index + 1).toLocaleString()} of {results.total.toLocaleString()}
        </p>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <Button
          size="sm"
          variant="outline"
          disabled={previous === null}
          onclick={() => move(previous)}
        >
          Previous
        </Button>
        <Button size="sm" variant="outline" disabled={next === null} onclick={() => move(next)}>
          Next
        </Button>
        <Button size="sm" variant="ghost" onclick={() => dialog?.close()}>Close</Button>
      </div>
    </header>

    <div class="flex min-h-0 flex-1 items-center justify-center bg-muted/30 p-3">
      {#if src}
        <img {src} alt={title} class="max-h-full max-w-full object-contain" />
      {:else}
        <p class="text-sm text-muted-foreground">Loading…</p>
      {/if}
    </div>

    {#if image && image.tags.length > 0}
      <footer class="border-t border-border px-3 py-2">
        <p class="text-xs wrap-break-word text-muted-foreground">{image.tags.join(' ')}</p>
      </footer>
    {/if}
  </div>
</dialog>
