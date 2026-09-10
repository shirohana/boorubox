<script lang="ts">
  // Slot Toolbar · view (design D17): the order and the grouping, beside the
  // tile-size slider. Both are part of the search request, so choosing one
  // re-runs the query rather than re-arranging what is on screen (design D6,
  // D7).
  import type { GroupBy, Sort } from '@boorubox/shared'
  import * as Select from '$lib/components/ui/select'

  interface Props {
    sort: Sort
    group: GroupBy
    /** The trash offers its own order and nothing else differs (`trash` D16). */
    view: 'library' | 'trash'
    onsort: (sort: Sort) => void
    ongroup: (group: GroupBy) => void
  }

  let { sort, group, view, onsort, ongroup }: Props = $props()

  // One list rather than a field select and a direction toggle: eight named
  // orders read faster in a toolbar than a field plus an arrow whose meaning
  // changes with it ("ascending file size" against "smallest first").
  const SORTS: { key: string, label: string, sort: Sort, view?: 'trash' }[] = [
    { key: 'trashed-desc', label: 'Trashed last', sort: { field: 'trashed', direction: 'desc' }, view: 'trash' },
    { key: 'trashed-asc', label: 'Trashed first', sort: { field: 'trashed', direction: 'asc' }, view: 'trash' },
    { key: 'captured-desc', label: 'Newest capture', sort: { field: 'captured', direction: 'desc' } },
    { key: 'captured-asc', label: 'Oldest capture', sort: { field: 'captured', direction: 'asc' } },
    { key: 'updated-desc', label: 'Changed last', sort: { field: 'updated', direction: 'desc' } },
    { key: 'updated-asc', label: 'Changed first', sort: { field: 'updated', direction: 'asc' } },
    { key: 'size-desc', label: 'Largest file', sort: { field: 'size', direction: 'desc' } },
    { key: 'size-asc', label: 'Smallest file', sort: { field: 'size', direction: 'asc' } },
    { key: 'dimensions-desc', label: 'Most pixels', sort: { field: 'dimensions', direction: 'desc' } },
    { key: 'dimensions-asc', label: 'Fewest pixels', sort: { field: 'dimensions', direction: 'asc' } },
  ]

  const GROUPS: { key: GroupBy, label: string }[] = [
    { key: 'none', label: 'No grouping' },
    { key: 'x-account', label: 'By X account' },
    { key: 'duplicates', label: 'By duplicates' },
  ]

  const offered = $derived(SORTS.filter((option) => !option.view || option.view === view))
  const sortKey = $derived(`${sort.field}-${sort.direction}`)
  const sortLabel = $derived(SORTS.find((option) => option.key === sortKey)?.label ?? '')
  const groupLabel = $derived(GROUPS.find((option) => option.key === group)?.label ?? '')
</script>

<Select.Root
  type="single"
  value={sortKey}
  onValueChange={(key) => {
    const picked = SORTS.find((option) => option.key === key)
    if (picked) onsort(picked.sort)
  }}
>
  <Select.Trigger size="sm" class="w-36 shrink-0" aria-label="Sort">{sortLabel}</Select.Trigger>
  <Select.Content>
    {#each offered as option (option.key)}
      <Select.Item value={option.key} label={option.label}>{option.label}</Select.Item>
    {/each}
  </Select.Content>
</Select.Root>

<Select.Root
  type="single"
  value={group}
  onValueChange={(key) => ongroup(key as GroupBy)}
>
  <Select.Trigger size="sm" class="w-32 shrink-0" aria-label="Group">{groupLabel}</Select.Trigger>
  <Select.Content>
    {#each GROUPS as option (option.key)}
      <Select.Item value={option.key} label={option.label}>{option.label}</Select.Item>
    {/each}
  </Select.Content>
</Select.Root>
