<script lang="ts">
  // Design D9: a `TagSidebar`-shaped list fed by the result's own groups.
  // Rust already orders them largest first, so this never sorts what it is
  // given.
  import type { GroupSlice } from '@boorubox/shared'
  import MinusIcon from '@lucide/svelte/icons/minus'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import {
    activeTerms,
    addAccountToQuery,
    excludeAccountFromQuery,
    toggleAccountInQuery,
  } from '$lib/domain/tag-utils'

  interface Props {
    groups: GroupSlice[]
    tagQuery: string
    onquery: (next: string) => void
  }

  let { groups, tagQuery, onquery }: Props = $props()

  // Design D3: the one reader for "is this term active", shared with the
  // tag sidebar and the inspector.
  const terms = $derived(activeTerms(tagQuery))
</script>

<div class="p-2">
  <h2 class="px-1 pb-1 text-xs font-medium text-muted-foreground">Accounts</h2>

  <ul class="flex flex-col gap-0.5">
    {#each groups as { key, count } (key)}
      <li
        class="
          flex items-center gap-1 rounded-md px-1 text-xs
          hover:bg-sidebar-accent
          {terms.accounts.has(key)
            ? 'bg-sky-500/15 font-medium text-sky-700 dark:text-sky-300'
            : ''}
          {terms.excludedAccounts.has(key) ? 'bg-destructive/10 text-destructive line-through' : ''}
        "
      >
        <button
          type="button"
          class="shrink-0 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
          aria-label="Include {key}"
          title="Include {key}"
          onclick={() => onquery(addAccountToQuery(tagQuery, key))}
        >
          <PlusIcon class="size-3" />
        </button>
        <button
          type="button"
          class="shrink-0 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
          aria-label="Exclude {key}"
          title="Exclude {key}"
          onclick={() => onquery(excludeAccountFromQuery(tagQuery, key))}
        >
          <MinusIcon class="size-3" />
        </button>
        <!-- Clicking an active account takes it out again (spec `sort-and-group`,
             "Undo from the rail"). -->
        <button
          type="button"
          class="min-w-0 flex-1 truncate py-1 text-left"
          onclick={() => onquery(toggleAccountInQuery(tagQuery, key))}
        >
          {key}
        </button>
        <span class="shrink-0 text-muted-foreground tabular-nums">{count}</span>
      </li>
    {/each}
  </ul>
</div>
