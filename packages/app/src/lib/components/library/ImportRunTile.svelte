<script lang="ts">
  import type { ImportRun } from '$lib/api'
  import { BUNDLE_COUNTING_LABEL } from '$lib/components/import/bundle-report'
  import { Progress } from '$lib/components/ui/progress'

  let { run }: { run: ImportRun } = $props()

  // Paths, not files: what was dropped is a list of items, and how many files
  // they hold is not known until Rust has walked them. A bundle run picks
  // `.db` files directly, so its count is exact from the start.
  const count = $derived(run.kind === 'paths' ? run.paths.length : run.files.length)
  const noun = $derived(run.kind === 'paths' ? 'item' : 'file')
  const items = $derived(`${count.toLocaleString()} ${noun}${count === 1 ? '' : 's'}`)
</script>

<div
  class="
    flex aspect-square flex-col justify-center gap-2 rounded-lg border border-dashed border-border
    bg-muted/40 p-3 text-xs
  "
  role="status"
>
  {#if run.status === 'queued'}
    <p class="font-medium">Waiting</p>
    <p class="text-muted-foreground tabular-nums">{items}</p>
  {:else if run.progress}
    <p class="font-medium tabular-nums">
      {run.progress.done.toLocaleString()} of {run.progress.total.toLocaleString()}
    </p>
    <Progress value={run.progress.done} max={run.progress.total} />
    <p class="text-muted-foreground tabular-nums">
      {run.progress.imported.toLocaleString()} imported
    </p>
  {:else if run.kind === 'bundle'}
    <p class="text-muted-foreground">{BUNDLE_COUNTING_LABEL}</p>
  {:else}
    <p class="text-muted-foreground">Looking through what you dropped…</p>
  {/if}
</div>
