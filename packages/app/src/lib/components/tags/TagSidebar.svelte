<script lang="ts">
  // Slot Sidebar · filters (`tags-and-ratings` design D17): what the current result
  // set is made of. The counts come from SQL over the whole result
  // (`tags-and-ratings` design D8), zero rows for the query's own tags included
  // (`tag-panel-polish` design D8) — the only thing computed here is the order:
  // the search's own tags first, then the rest (`tag-panel-polish` D7, amended
  // 2026-09-23 from the running app — no group labels, the colour carries the
  // grouping, and a count or category order under the search's own tags read
  // as noise once the labels were gone).
  import type { TagCount } from '@boorubox/shared'
  import MinusIcon from '@lucide/svelte/icons/minus'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import { vocabulary } from '$lib/api'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { groupByCategory } from '$lib/domain/tag-categories'
  import {
    activeTerms,
    addTagToQuery,
    excludeTagFromQuery,
    toggleTagInQuery,
  } from '$lib/domain/tag-utils'
  import { CATEGORY_TEXT_CLASS, searchMark, searchMarkClass } from './categories'
  import TagVocabularyMenuItems from './TagVocabularyMenuItems.svelte'

  interface Props {
    /**
     * `null` while a search is running (`tags-and-ratings` design D8): the
     * heading stays, the list is blank.
     */
    tags: TagCount[] | null
    tagQuery: string
    onquery: (next: string) => void
  }

  let { tags, tagQuery, onquery }: Props = $props()

  // `inspector-polish` design D3: the one reader for "is this term active", shared
  // with the inspector's tags.
  const terms = $derived(activeTerms(tagQuery))
  const included = $derived(terms.included)
  const excluded = $derived(terms.excluded)

  /**
   * The list's order (design D7, amended `tag-panel-polish` 2026-09-23): the
   * tags the search includes or excludes first, then the rest — each half
   * through `groupByCategory` (artist, copyright, character, general, meta,
   * alphabetical inside), so "category order, alphabetical inside" is
   * written once and applied to both halves rather than re-typed as a
   * second comparator here. No group labels: the colour carries the
   * grouping, same as the inspector. `tags` already carries a zero row for
   * every name the query includes or excludes that the result does not, and
   * leaves out a name no tag has (`tag-panel-polish` design D8) — Rust is
   * the one place that can tell the two apart, so nothing here merges the
   * query back in.
   */
  const rows = $derived.by(() => {
    if (!tags) return null
    const isActive = (row: TagCount) => included.has(row.name) || excluded.has(row.name)
    const ordered = (part: TagCount[]) =>
      groupByCategory(part, (row) => row.name, vocabulary.categoryOf)
        .flatMap((group) => group.items)
    return [...ordered(tags.filter(isActive)), ...ordered(tags.filter((row) => !isActive(row)))]
  })
</script>

<!--
  The sidebar's one flexible section (`sidebar-layout` design D1): it takes
  whatever height the other sections leave and scrolls on its own. `min-h-32`
  is its floor (`browse-feedback`): the collections box below has a height of
  its own, and without a floor here a short window gave the tag list nothing
  at all — past the floor it is the whole sidebar that scrolls, which reads
  as "too short" rather than "no tags".
  `data-sidebar="tags"` is the hook app.css scrolls it by — its rules sit
  beside the copy-in content wrapper's, which cannot be reached from a class
  here. Dropping the attribute drops the scrollbar gutter and the
  sideways-scroll clip with it.
-->
<section data-sidebar="tags" class="min-h-32 flex-1 overflow-y-auto p-2">
  <h2 class="px-1 pb-1 text-xs font-medium text-muted-foreground">Tags</h2>

  {#if rows && rows.length === 0}
    <p class="px-1 text-xs text-muted-foreground">No tags in these results</p>
  {:else if rows}
    <!-- No gap and half the padding: the owner wants more rows on screen (2026-09-23). -->
    <ul class="flex flex-col">
      {#each rows as { name, count } (name)}
        <li>
          <ContextMenu.Root>
            <ContextMenu.Trigger>
              {#snippet child({ props })}
                <!--
                  The search marking (design D3, amended `tag-panel-polish` 2026-09-23)
                  sits on the whole row — the hoverable box, not just the name — same
                  as it did before unit C's pass moved it onto the name alone.
                  `searchMarkClass` (amended again 2026-09-23) supplies
                  `hover:bg-sidebar-accent` only for a row with no mark, so an
                  active or excluded row keeps its own hover instead of losing
                  it to a competing neutral one at equal specificity.
                -->
                <div
                  {...props}
                  class="
                    flex items-center gap-1 rounded-md px-1 text-xs
                    {searchMarkClass(searchMark(name, included, excluded), 'hover:bg-sidebar-accent')}
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
                  <!--
                    Clicking an active tag takes it out again (spec `tag-sidebar`). The
                    category colour is the vocabulary's, read fresh on every render
                    (`tag-vocabulary` design D5) — the name carries only that colour now;
                    the search marking is the row's, above.
                  -->
                  <button
                    type="button"
                    class="
                      min-w-0 flex-1 truncate py-0.5 text-left
                      {CATEGORY_TEXT_CLASS[vocabulary.categoryOf(name)]}
                    "
                    onclick={() => onquery(toggleTagInQuery(tagQuery, name))}
                  >
                    {name}
                  </button>
                  <span class="shrink-0 text-muted-foreground tabular-nums">{count}</span>
                </div>
              {/snippet}
            </ContextMenu.Trigger>
            <ContextMenu.Content>
              <TagVocabularyMenuItems {name} />
            </ContextMenu.Content>
          </ContextMenu.Root>
        </li>
      {/each}
    </ul>
  {/if}
</section>
