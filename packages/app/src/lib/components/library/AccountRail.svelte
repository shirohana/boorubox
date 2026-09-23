<script lang="ts">
  // A `TagSidebar`-shaped list of every account in the view (`account-rail-
  // counts` design D2), counted with the search's own account terms set aside
  // — the rating pills' sideways question, so an included account leaves the
  // others listed with what they would give. Rust orders largest first; the
  // only sort here moves the search's own accounts to the front, stable, so
  // that order holds within each half.
  import type { GroupSlice } from '@boorubox/shared'
  import MinusIcon from '@lucide/svelte/icons/minus'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import { searchMark, searchMarkClass } from '$lib/components/tags/categories'
  import {
    activeTerms,
    addAccountToQuery,
    excludeAccountFromQuery,
    toggleAccountInQuery,
  } from '$lib/domain/tag-utils'

  interface Props {
    /** `null` while a search runs: the rows stay, their numbers go blank. */
    accounts: GroupSlice[] | null
    tagQuery: string
    onquery: (next: string) => void
  }

  let { accounts, tagQuery, onquery }: Props = $props()

  // Design D3: the one reader for "is this term active", shared with the
  // tag sidebar and the inspector.
  const terms = $derived(activeTerms(tagQuery))
  const isActive = (handle: string) =>
    terms.accounts.has(handle) || terms.excludedAccounts.has(handle)

  /**
   * The last answer's rows, kept through the next search: counts are `null`
   * for the length of every search, and rows drawn from `null` unmount, which
   * throws the rail's scroll position away under the row just clicked — the
   * reset `CollectionsSection` had. The numbers are blank meanwhile.
   */
  let lastHandles: string[] = []
  const rows = $derived.by((): { handle: string, count: number | null }[] => {
    if (!accounts) return lastHandles.map((handle) => ({ handle, count: null }))
    lastHandles = accounts.map((account) => account.key)
    return accounts.map((account) => ({ handle: account.key, count: account.count }))
  })
  const listed = $derived(
    [...rows].sort((a, b) => Number(isActive(b.handle)) - Number(isActive(a.handle))),
  )
</script>

<div class="p-2">
  <h2 class="px-1 pb-1 text-xs font-medium text-muted-foreground">Accounts</h2>

  <ul class="flex flex-col gap-0.5">
    {#each listed as { handle: key, count } (key)}
      <!--
        The row reads the same table the tag sidebar and the inspector do
        (design D3): background only, no strike-through, the shared emerald
        tint for an included handle rather than a sky one of its own — a
        second "active" colour beside the tags' would have said something
        different about the same state. `searchMarkClass` supplies
        `hover:bg-sidebar-accent` only while the row carries no mark, so an
        active or excluded row keeps its own hover instead of losing it to a
        competing neutral one.
      -->
      <li
        class="
          flex items-center gap-1 rounded-md px-1 text-xs
          {searchMarkClass(searchMark(key, terms.accounts, terms.excludedAccounts), 'hover:bg-sidebar-accent')}
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
        <span class="shrink-0 text-muted-foreground tabular-nums">{count ?? ''}</span>
      </li>
    {/each}
  </ul>
</div>
