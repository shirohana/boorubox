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
  import {
    isTypingTarget,
    KEY_INSPECT,
    KEY_LEFT,
    KEY_RIGHT,
    KEY_SPACE,
  } from '$lib/keyboard'
  import Inspector from './Inspector.svelte'

  interface Props {
    results: SearchResults
    index: number
    libraryPath: string | null
    /** Forwarded to the inspector, which edits here exactly as it does beside the grid. */
    tagQuery: string
    onquery: (next: string) => void
    onclose: () => void
  }

  let { results, index = $bindable(), libraryPath, tagQuery, onquery, onclose }: Props = $props()

  let dialog = $state<HTMLDialogElement | null>(null)
  // Which mode the viewer opens in is session state, not a setting (Non-Goals),
  // and the session is this dialog: it starts on the image alone every time.
  let mode = $state<'gallery' | 'inspect'>('gallery')

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
    if (isTypingTarget(event)) return

    if (event.key === KEY_LEFT) {
      event.preventDefault()
      move(previous)
    } else if (event.key === KEY_RIGHT) {
      event.preventDefault()
      move(next)
    } else if (event.key === KEY_INSPECT) {
      event.preventDefault()
      mode = mode === 'inspect' ? 'gallery' : 'inspect'
    } else if (event.key === KEY_SPACE) {
      // Escape is the dialog's own; Space is not, so it has to ask.
      event.preventDefault()
      dialog?.close()
    }
  }
</script>

<dialog
  bind:this={dialog}
  {onclose}
  {onkeydown}
  class="
    m-auto h-[96vh] max-h-none w-[96vw] max-w-none border-0 bg-transparent p-0
    backdrop:bg-black/85
  "
>
  <div class="flex h-full min-h-0 gap-3">
    <div class="flex min-w-0 flex-1 flex-col">
      <!-- Minimal chrome: the image is what the viewer is for. -->
      <!-- The dialog reaches the window's top edge, where the traffic lights are (D13). -->
      <header
        class="
          flex items-center justify-between gap-3 px-1 py-2 text-white
          in-data-[platform=macos]:ps-16
        "
      >
        <div class="min-w-0">
          <p class="truncate text-sm">{title}</p>
          <p class="text-xs text-white/60">
            {(index + 1).toLocaleString()} of {results.total.toLocaleString()}
          </p>
        </div>
        <div class="flex shrink-0 items-center gap-1">
          <Button
            size="sm"
            variant="ghost"
            class="text-white hover:bg-white/15 hover:text-white"
            disabled={previous === null}
            onclick={() => move(previous)}
          >
            Previous
          </Button>
          <Button
            size="sm"
            variant="ghost"
            class="text-white hover:bg-white/15 hover:text-white"
            disabled={next === null}
            onclick={() => move(next)}
          >
            Next
          </Button>
          <Button
            size="sm"
            variant="ghost"
            class="text-white hover:bg-white/15 hover:text-white"
            aria-pressed={mode === 'inspect'}
            onclick={() => (mode = mode === 'inspect' ? 'gallery' : 'inspect')}
          >
            Info
          </Button>
          <Button
            size="sm"
            variant="ghost"
            class="text-white hover:bg-white/15 hover:text-white"
            onclick={() => dialog?.close()}
          >
            Close
          </Button>
        </div>
      </header>

      <!--
        The image is fitted to whatever space is left, so opening the inspector
        refits it rather than cropping it (design D10).
      -->
      <div class="flex min-h-0 min-w-0 flex-1 items-center justify-center">
        {#if src}
          <img {src} alt={title} class="max-h-full max-w-full object-contain" />
        {:else}
          <p class="text-sm text-white/60">Loading…</p>
        {/if}
      </div>
    </div>

    {#if mode === 'inspect'}
      <aside
        class="
          my-2 w-80 shrink-0 overflow-hidden rounded-xl border border-border bg-background
          text-foreground
        "
      >
        <Inspector image={image ?? null} {results} {tagQuery} {onquery} />
      </aside>
    {/if}
  </div>
</dialog>
