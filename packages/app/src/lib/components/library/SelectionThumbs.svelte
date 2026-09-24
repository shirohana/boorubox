<script lang="ts">
  // The inspector's multi-selection strip (design D7). A thumbnail needs only an
  // id, so this draws whatever ids it is handed without loading a record for
  // any of them — the count beside it is the number that has to be right.
  //
  // Every thumbnail is a control, not a picture (D7, amended): the strip was
  // shipped as a read-only preview and the owner found a panel that shows the
  // selection and can do nothing with it. Pressing one opens it in the viewer;
  // its own button drops that one image out of the selection.
  import XIcon from '@lucide/svelte/icons/x'
  import { tick } from 'svelte'
  import { cachedThumbnail, cacheEpoch, thumbnail } from './thumbnail-cache.svelte'

  interface Props {
    ids: string[]
    /** Selected images the strip is not showing; `0` draws nothing. */
    more: number
    /** Open this image in the screen's viewer. */
    onopen: (id: string) => void
    /** Take this one image out of the selection, leaving the rest. */
    onremove: (id: string) => void
  }

  let { ids, more, onopen, onremove }: Props = $props()

  /**
   * The × unmounts with its cell, and a focus left on an unmounted button falls
   * to the body — the next Enter then does nothing and no screen shortcut
   * reaches the grid. So the focus moves to the thumbnail that takes this
   * cell's place (or the last one left) once the strip has redrawn.
   */
  async function removeAndStay(button: HTMLElement, id: string) {
    const strip = button.closest('ul')
    const cell = button.closest('li')
    const at = strip && cell ? [...strip.children].indexOf(cell) : -1
    onremove(id)
    await tick()
    if (!strip || at < 0) return
    const cells = strip.querySelectorAll('li')
    cells[Math.min(at, cells.length - 1)]?.querySelector('button')?.focus()
  }

  let urls = $state<Record<string, string>>({})

  // `cachedThumbnail` rather than `urls` decides what still has to be asked for:
  // reading `urls` here would make this effect depend on its own writes.
  $effect(() => {
    // Read so a `forgetAll()` (design D6) re-runs this effect for ids already
    // drawn — otherwise the strip never asks again once a card is on screen.
    // Stale entries in `urls` are left as they are below when the round trip
    // has to repeat, so a tile keeps its old thumbnail rather than flashing
    // empty while it reloads.
    cacheEpoch()
    let current = true
    for (const id of ids) {
      const known = cachedThumbnail(id)
      if (known !== null) {
        urls[id] = known
        continue
      }
      void thumbnail(id)
        .then((url) => {
          if (current) urls[id] = url
        })
        .catch(() => {})
    }
    return () => {
      current = false
    }
  })
</script>

<ul class="grid grid-cols-4 gap-1.5">
  {#each ids as id (id)}
    <li class="group/thumb relative aspect-square">
      <button
        type="button"
        onclick={() => onopen(id)}
        aria-label="Open this image"
        class="
          block size-full overflow-hidden rounded-md border border-border bg-muted/40 outline-none
          focus-visible:ring-3 focus-visible:ring-ring
        "
      >
        {#if urls[id]}
          <img src={urls[id]} alt="" draggable="false" class="size-full object-contain" />
        {/if}
      </button>

      <!--
        Outside the thumbnail's button rather than inside it: a control nested in
        a button is not reachable on its own. Hidden until the thumbnail is
        hovered or something in the cell has the focus — `opacity`, not
        `hidden`, so it is still in the tab order and shows itself when tabbed
        to.
      -->
      <button
        type="button"
        onclick={(event) => removeAndStay(event.currentTarget, id)}
        aria-label="Remove this image from the selection"
        title="Remove from the selection"
        class="
          absolute -top-1 -right-1 flex size-5 items-center justify-center rounded-full border
          border-border bg-background text-muted-foreground opacity-0 transition-opacity
          group-hover/thumb:opacity-100
          hover:text-foreground
          focus-visible:opacity-100 focus-visible:ring-3 focus-visible:ring-ring
          focus-visible:outline-none
        "
      >
        <XIcon class="size-3" />
      </button>
    </li>
  {/each}
</ul>

{#if more > 0}
  <p class="mt-2 text-xs text-muted-foreground">+{more.toLocaleString()} more</p>
{/if}
