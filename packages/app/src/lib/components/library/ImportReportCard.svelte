<script lang="ts">
  // Under the toolbar, not inside it: the per-item list is the only place a
  // skipped or failed file is ever named, so it stays until it is dismissed
  // rather than vanishing with the menu that started the run (design D8).
  import type { ImportReport } from '@boorubox/shared'
  import { Button } from '$lib/components/ui/button'

  let { report, ondismiss }: { report: ImportReport, ondismiss: () => void } = $props()

  const unimported = $derived(report.items.filter((item) => item.status !== 'imported'))
</script>

<div class="border-b border-border px-4 py-2 text-xs">
  <div class="flex items-baseline justify-between gap-2">
    <p>
      Imported {report.imported.toLocaleString()} · skipped
      {report.skipped.toLocaleString()} · failed {report.failed.toLocaleString()}
    </p>
    <Button size="xs" variant="ghost" onclick={ondismiss}>Dismiss</Button>
  </div>
  {#if unimported.length > 0}
    <ul class="mt-1 max-h-32 overflow-y-auto text-muted-foreground">
      {#each unimported as item (item.path)}
        <li class="break-all">
          <span class="font-medium">{item.status}</span>
          {item.path}{item.reason ? ` — ${item.reason}` : ''}
        </li>
      {/each}
    </ul>
  {/if}
</div>
