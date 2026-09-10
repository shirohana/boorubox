<script lang="ts">
  // Under the toolbar beside the import reports, not inside the selection
  // toolbar: a missing image is named here and nowhere else, and clearing the
  // selection — the obvious next thing to do — must not take the report with it
  // (spec `export-selected`).
  import type { ExportReport } from '@boorubox/shared'
  import { Button } from '$lib/components/ui/button'

  let { report, ondismiss }: { report: ExportReport, ondismiss: () => void } = $props()
</script>

<div class="border-b border-border px-4 py-2 text-xs">
  <div class="flex items-baseline justify-between gap-2">
    <p class="break-all">
      {#if report.written === 0}
        Nothing was written to {report.path}: every selected image's file is missing.
      {:else}
        Exported {report.written.toLocaleString()} images to {report.path}.
      {/if}
    </p>
    <Button size="xs" variant="ghost" onclick={ondismiss}>Dismiss</Button>
  </div>

  {#if report.missing.length > 0}
    <ul class="mt-1 max-h-32 overflow-y-auto text-muted-foreground">
      {#each report.missing as id (id)}
        <li class="font-mono break-all">{id} — file missing from the library folder</li>
      {/each}
    </ul>
  {/if}
</div>
