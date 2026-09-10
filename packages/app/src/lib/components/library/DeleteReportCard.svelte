<script lang="ts">
  // Design D4: the rows are gone and one of the files would not go. Nothing in
  // the library refers to that file any more and the delete cannot be re-run,
  // so its full path is put in front of the user rather than into a log — the
  // app says what it could not do and where, and the user decides.
  //
  // Beside the import and export reports rather than inside the toolbar: the
  // selection it came from is cleared by the delete, and the report has to
  // outlive it.
  import type { DeleteReport } from '@boorubox/shared'
  import { Button } from '$lib/components/ui/button'

  let { report, ondismiss }: { report: DeleteReport, ondismiss: () => void } = $props()
</script>

<div class="border-b border-border px-4 py-2 text-xs">
  <div class="flex items-baseline justify-between gap-2">
    <p class="break-all">
      Deleted {report.deleted.toLocaleString()}
      {report.deleted === 1 ? 'image' : 'images'}.
      {report.filesLeft.length.toLocaleString()}
      {report.filesLeft.length === 1 ? 'file' : 'files'} could not be removed and can be deleted
      by hand:
    </p>
    <Button size="xs" variant="ghost" onclick={ondismiss}>Dismiss</Button>
  </div>

  <ul class="mt-1 max-h-32 overflow-y-auto text-muted-foreground">
    {#each report.filesLeft as path (path)}
      <li class="font-mono break-all">{path}</li>
    {/each}
  </ul>
</div>
