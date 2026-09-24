<script lang="ts">
  // One row, shared by the tag list and the collection list
  // (`sidebar-inspector-polish` design D7): the two were the same markup
  // twice, drifting a little further apart each time one of them changed.
  // No behaviour lives here — include, exclude and toggle stay the caller's;
  // this only draws them, so a name, a count, a search mark and a context
  // menu are all either component ever needs to supply.
  import type { Snippet } from 'svelte'
  import MinusIcon from '@lucide/svelte/icons/minus'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { searchMarkClass, type SearchMark } from './categories'

  interface Props {
    name: string
    /** The collection list passes `''` while a search runs (`collections` design D8's
     * honest blank). */
    count: number | string
    mark: SearchMark
    /** The tag list's category colour; the collection list passes nothing. */
    nameClass?: string
    oninclude: () => void
    onexclude: () => void
    ontoggle: () => void
    menu: Snippet
  }

  let { name, count, mark, nameClass = '', oninclude, onexclude, ontoggle, menu }: Props = $props()
</script>

<li>
  <ContextMenu.Root>
    <ContextMenu.Trigger>
      {#snippet child({ props })}
        <div
          {...props}
          class="
            flex items-center gap-1 rounded-md px-1 text-xs
            {searchMarkClass(mark, 'hover:bg-sidebar-accent')}
          "
        >
          <button
            type="button"
            class="shrink-0 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
            aria-label="Include {name}"
            title="Include {name}"
            onclick={oninclude}
          >
            <PlusIcon class="size-3" />
          </button>
          <button
            type="button"
            class="shrink-0 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
            aria-label="Exclude {name}"
            title="Exclude {name}"
            onclick={onexclude}
          >
            <MinusIcon class="size-3" />
          </button>
          <!-- Clicking an active row takes it out again (spec "Filter from the list"). -->
          <button
            type="button"
            class="min-w-0 flex-1 truncate py-0.5 text-left {nameClass}"
            onclick={ontoggle}
          >
            {name}
          </button>
          <span class="shrink-0 text-muted-foreground tabular-nums">{count}</span>
        </div>
      {/snippet}
    </ContextMenu.Trigger>
    <ContextMenu.Content>
      {@render menu()}
    </ContextMenu.Content>
  </ContextMenu.Root>
</li>
