<script lang="ts">
  // Under the toolbar, not inside it: the per-item list is the only place a
  // skipped or failed file is ever named, so it stays until it is dismissed
  // rather than vanishing with the menu that started the run (design D8).
  import type { ImportReportEntry } from '$lib/api'
  import { queuedDiscardedNotice, rerunNotice } from '$lib/components/import/cancelled-import'
  import { Button } from '$lib/components/ui/button'

  let { report, ondismiss }: { report: ImportReportEntry, ondismiss: () => void } = $props()

  const unimported = $derived(report.items.filter((item) => item.status !== 'imported'))
  // Gated on `cancelled`: a Cancel that lands just after an ordinary run
  // finishes on its own still drops the queue, but that report is not the
  // cancelled run's own and should not carry its line.
  const queuedNotice = $derived(
    report.cancelled ? queuedDiscardedNotice(report.queuedDiscarded) : null,
  )
</script>

<div class="border-b border-border px-4 py-2 text-xs">
  <div class="flex items-baseline justify-between gap-2">
    <p>
      {#if report.cancelled}<span class="font-medium">Cancelled</span> — {/if}Imported
      {report.imported.toLocaleString()} · skipped
      {report.skipped.toLocaleString()} · failed {report.failed.toLocaleString()}
    </p>
    <Button size="xs" variant="ghost" onclick={ondismiss}>Dismiss</Button>
  </div>
  {#if report.cancelled}
    <!--
      Not styled or worded as an error (`pending-work` spec "Cancelling is not
      failing"), and no Retry that would repeat the whole run silently — this
      card has never offered one.
    -->
    <p class="mt-1 text-muted-foreground">{rerunNotice(report.kind)}</p>
  {/if}
  {#if queuedNotice}
    <p class="mt-1 text-muted-foreground">{queuedNotice}</p>
  {/if}
  {#if unimported.length > 0}
    <ul class="mt-1 max-h-32 overflow-y-auto text-muted-foreground">
      {#each unimported as item, index (index)}
        <li class="break-all">
          <span class="font-medium">{item.status}</span>
          {item.path}{item.reason ? ` — ${item.reason}` : ''}
        </li>
      {/each}
    </ul>
  {/if}
</div>
