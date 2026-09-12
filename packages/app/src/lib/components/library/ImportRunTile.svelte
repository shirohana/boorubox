<script lang="ts">
  import type { ImportRun } from '$lib/api'
  import { imports } from '$lib/api'
  import { BUNDLE_COUNTING_LABEL } from '$lib/components/import/bundle-report'
  import { Button } from '$lib/components/ui/button'
  import { Progress } from '$lib/components/ui/progress'

  let { run }: { run: ImportRun } = $props()

  // Paths, not files: what was dropped is a list of items, and how many files
  // they hold is not known until Rust has walked them. A bundle run picks
  // `.db` files directly, so its count is exact from the start.
  const count = $derived(run.kind === 'paths' ? run.paths.length : run.files.length)
  const noun = $derived(run.kind === 'paths' ? 'item' : 'file')
  const items = $derived(`${count.toLocaleString()} ${noun}${count === 1 ? '' : 's'}`)
  const countingLabel = $derived(
    run.kind === 'bundle' ? BUNDLE_COUNTING_LABEL : 'Looking through what you dropped…',
  )
  // `imports.paused` is a single flag for the one run that can ever be
  // parked (`import-pause-cancel` design D1) — only the running tile reads
  // it as its own state, so a queued tile that outlives a pause never shows
  // as paused.
  const paused = $derived(run.status === 'running' && imports.paused)
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
  {:else if paused}
    <!-- Says it is paused rather than looking stalled (`pending-work` spec). -->
    <p class="font-medium">Paused</p>
    {#if run.progress}
      <p class="text-muted-foreground tabular-nums">
        {run.progress.done.toLocaleString()} of {run.progress.total.toLocaleString()}
      </p>
    {/if}
  {:else if run.progress}
    <p class="font-medium tabular-nums">
      {run.progress.done.toLocaleString()} of {run.progress.total.toLocaleString()}
    </p>
    <Progress value={run.progress.done} max={run.progress.total} />
    <p class="text-muted-foreground tabular-nums">
      {run.progress.imported.toLocaleString()} imported
    </p>
  {:else}
    <p class="text-muted-foreground">{countingLabel}</p>
  {/if}

  <div class="mt-auto flex gap-1">
    {#if run.status === 'running' && !paused}
      <Button size="xs" variant="ghost" onclick={() => imports.pause()}>Pause</Button>
    {:else if paused}
      <Button size="xs" variant="ghost" onclick={() => imports.resume()}>Resume</Button>
    {/if}
    <!--
      Every tile offers Cancel (`pending-work` spec "A waiting import SHALL
      offer Cancel"), but they mean different things: the running tile's
      Cancel stops the run and drops the whole queue behind it (design D6); a
      waiting tile's only ever removes itself.
    -->
    <Button
      size="xs"
      variant="ghost"
      onclick={() => (run.status === 'queued' ? imports.dequeue(run.id) : imports.cancel())}
    >
      Cancel
    </Button>
  </div>
</div>
