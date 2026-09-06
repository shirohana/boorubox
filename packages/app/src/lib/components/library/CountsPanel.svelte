<script lang="ts">
  // These counts are the whole library, never the current search: they are what
  // makes a migration verifiable (spec `library-browse`), so a search must not
  // move them.
  import type { ImageCounts } from '@boorubox/shared'

  let { counts }: { counts: ImageCounts | null } = $props()

  const rows = $derived(counts
    ? [
      { label: 'total', value: counts.total },
      { label: 'extension', value: counts.extension },
      { label: 'local', value: counts.local },
      { label: 'legacy bundle', value: counts.legacyBundle },
    ]
    : [])
</script>

{#if counts}
  <div class="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-xs text-muted-foreground">
    <span class="font-medium text-foreground">Whole library</span>
    <dl class="flex flex-wrap items-baseline gap-x-4 gap-y-1">
      {#each rows as row (row.label)}
        <div class="flex items-baseline gap-1">
          <dt>{row.label}</dt>
          <dd class="font-medium text-foreground tabular-nums">{row.value.toLocaleString()}</dd>
        </div>
      {/each}
    </dl>
  </div>
{/if}
