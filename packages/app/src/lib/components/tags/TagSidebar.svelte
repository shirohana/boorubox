<script lang="ts">
  // Slot Sidebar · filters (design D17): what the current result set is made of.
  // The counts come from SQL over the whole result (design D8); the only thing
  // computed here is the order, and the tags the query names that the result
  // does not carry — the parsed query is in the webview, and so is this.
  import type { TagCount } from '@boorubox/shared'
  import MinusIcon from '@lucide/svelte/icons/minus'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import {
    activeTerms,
    addTagToQuery,
    excludeTagFromQuery,
    toggleTagInQuery,
  } from '$lib/domain/tag-utils'

  interface Props {
    /** `null` while a search is running (design D8): the heading stays, the list is blank. */
    tags: TagCount[] | null
    tagQuery: string
    onquery: (next: string) => void
  }

  let { tags, tagQuery, onquery }: Props = $props()

  // Design D3: the one reader for "is this term active", shared with the
  // inspector's badges.
  const terms = $derived(activeTerms(tagQuery))
  const included = $derived(terms.included)
  const excluded = $derived(terms.excluded)

  /**
   * A tag the search names but the result does not carry is listed at zero, so
   * the filter that emptied the result can be undone from where it is shown.
   */
  const listed = $derived.by(() => {
    if (!tags) return null
    const counted = new Set(tags.map((tag) => tag.name))
    const missing = [...new Set([...included, ...excluded])]
      .filter((name) => !counted.has(name))
      .map((name) => ({ name, count: 0 }))
    const active = (name: string) => included.has(name) || excluded.has(name)

    return [...tags, ...missing].sort((a, b) => {
      if (active(a.name) !== active(b.name)) return active(a.name) ? -1 : 1
      if (a.count !== b.count) return b.count - a.count
      return a.name.localeCompare(b.name)
    })
  })
</script>

<section class="min-h-0 p-2">
  <h2 class="px-1 pb-1 text-xs font-medium text-muted-foreground">Tags</h2>

  {#if listed && listed.length === 0}
    <p class="px-1 text-xs text-muted-foreground">No tags in these results</p>
  {:else if listed}
    <ul class="flex flex-col gap-0.5">
      {#each listed as { name, count } (name)}
        <li
          class="
            flex items-center gap-1 rounded-md px-1 text-xs
            hover:bg-sidebar-accent
            {included.has(name)
              ? `bg-emerald-500/15 font-medium text-emerald-700 dark:text-emerald-300`
              : ''}
            {excluded.has(name) ? 'bg-destructive/10 text-destructive line-through' : ''}
          "
        >
          <button
            type="button"
            class="shrink-0 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
            aria-label="Include {name}"
            title="Include {name}"
            onclick={() => onquery(addTagToQuery(tagQuery, name))}
          >
            <PlusIcon class="size-3" />
          </button>
          <button
            type="button"
            class="shrink-0 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
            aria-label="Exclude {name}"
            title="Exclude {name}"
            onclick={() => onquery(excludeTagFromQuery(tagQuery, name))}
          >
            <MinusIcon class="size-3" />
          </button>
          <!-- Clicking an active tag takes it out again (spec `tag-sidebar`). -->
          <button
            type="button"
            class="min-w-0 flex-1 truncate py-1 text-left"
            onclick={() => onquery(toggleTagInQuery(tagQuery, name))}
          >
            {name}
          </button>
          <span class="shrink-0 text-muted-foreground tabular-nums">{count}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>
