<script lang="ts">
  // What a rebuild is doing, and what it did — the same two states wherever one
  // is asked for (`library-sidecars` design D12): the start screen's damaged
  // state, where this stands between the rebuild and the library opening, and
  // Settings → Library, where the library reopens behind it. One component
  // because spec `library-recovery` asks one thing of both — progress while it
  // runs rather than a screen that looks stalled, then how many images came
  // back, which files could not be read, and the name the previous database was
  // kept under.
  import type { RebuildReport } from '@boorubox/shared'
  import { Progress } from '$lib/components/ui/progress'

  interface Props {
    /** The command is in flight; `progress` stays null until the first tick. */
    running: boolean
    progress: { done: number, total: number } | null
    /** The finished run's report, which takes over from the progress above. */
    report: RebuildReport | null
  }

  let { running, progress, report }: Props = $props()
</script>

{#if report}
  <dl class="flex flex-col gap-1 text-sm">
    <div class="flex items-baseline justify-between gap-2">
      <dt class="text-muted-foreground">Images restored</dt>
      <dd class="font-medium tabular-nums">{report.images.toLocaleString()}</dd>
    </div>
    <!--
      Shown even at zero, which is the point: rules and sites come back from
      `library.json` alone (design D3), so a folder that lost that one file
      rebuilds every image and no rule at all. Two zeroes here is how the user
      finds that out now rather than the next time a rule does not fire.
    -->
    <div class="flex items-baseline justify-between gap-2">
      <dt class="text-muted-foreground">Rules, booru sites</dt>
      <dd class="font-medium tabular-nums">
        {report.rules.toLocaleString()}, {report.sites.toLocaleString()}
      </dd>
    </div>
    {#if report.failed > 0}
      <div class="flex items-baseline justify-between gap-2">
        <dt class="text-muted-foreground">Files that could not be read</dt>
        <dd class="font-medium tabular-nums">{report.failed.toLocaleString()}</dd>
      </div>
    {/if}
  </dl>

  {#if report.failures.length > 0}
    <!--
      Capped and scrolled like the export report's missing list: a rebuild that
      could not read a thousand files must not push the rest of the screen off
      it. Keyed by position, not by `file`: a report is replaced whole and
      never reordered, and two failures naming one file would take a list keyed
      on it down with them — which is the last thing the screen that explains
      the damage may do.
    -->
    <ul class="max-h-32 overflow-y-auto font-mono text-xs text-muted-foreground">
      {#each report.failures as failure, index (index)}
        <li class="break-all">{failure.file} — {failure.reason}</li>
      {/each}
    </ul>
  {/if}

  <!-- Empty when there was no database to keep: a folder that had lost
       library.sqlite altogether, which the rebuild exists to serve. -->
  {#if report.keptAs}
    <p class="text-xs text-muted-foreground">
      The previous database is kept as
      <span class="font-mono break-all">{report.keptAs}</span>.
    </p>
  {/if}
{:else if running}
  <p class="text-sm font-medium">Rebuilding…</p>
  {#if progress}
    <p class="text-sm text-muted-foreground tabular-nums">
      {progress.done.toLocaleString()} of {progress.total.toLocaleString()}
    </p>
    <Progress value={progress.done} max={progress.total} />
  {/if}
{/if}
