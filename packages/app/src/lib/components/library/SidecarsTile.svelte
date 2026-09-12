<script lang="ts">
  // The one tile for the sidecar catch-up pass (`library-sidecars` design D13,
  // `pending-work` spec). No controls: unlike an import, the user never
  // starts or stops this — it runs once at open and stops on its own on a
  // library switch (`sidecars-backfill.svelte.ts`).
  import { sidecarsBackfill } from '$lib/api'
  import { Progress } from '$lib/components/ui/progress'

  const progress = $derived(sidecarsBackfill.progress)
</script>

{#if progress}
  <div
    class="
      flex aspect-square flex-col justify-center gap-2 rounded-lg border border-dashed border-border
      bg-muted/40 p-3 text-xs
    "
    role="status"
  >
    <p class="font-medium">Catching up</p>
    <p class="text-muted-foreground tabular-nums">
      {progress.done.toLocaleString()} of {progress.total.toLocaleString()}
    </p>
    <Progress value={progress.done} max={progress.total} />
  </div>
{/if}
