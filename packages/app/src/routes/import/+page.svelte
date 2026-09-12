<script lang="ts">
  // Where a legacy-bundle run starts and where its report is read (design D7).
  // A route of its own, not a dialog off the sidebar: the run outlives it
  // (`Imports` is the singleton queue, design D9 of the local-import change),
  // so leaving and coming back still shows the same progress or report.
  import { bundlePick, imports, libraryCounts, pickBundleFiles } from '$lib/api'
  import {
    type BundleOutcomeGroup,
    BUNDLE_COUNTING_LABEL,
    BUNDLE_MIGRATION_NOTICE,
    bundleCountsLine,
    groupBundleReport,
  } from '$lib/components/import/bundle-report'
  import { queuedDiscardedNotice, rerunNotice } from '$lib/components/import/cancelled-import'
  import { Button } from '$lib/components/ui/button'
  import { windowDragRegion } from '$lib/platform'
  import { Progress } from '$lib/components/ui/progress'

  /** Rows shown per status before folding the rest behind "and N more". */
  const OUTCOME_PREVIEW_LIMIT = 200

  // The bundle run this route watches, if any: it may still be `queued`
  // behind an earlier paths run (design D6: one queue for every run kind), not
  // only `running`, so the markup below branches on `status` first.
  const bundleRun = $derived(imports.runs.find((run) => run.kind === 'bundle') ?? null)
  const groups = $derived(
    imports.latestBundleReport ? groupBundleReport(imports.latestBundleReport) : null,
  )
  // `imports.paused` describes whichever run is at the front of the queue,
  // which may not be this one — a bundle run sitting `queued` behind a paths
  // run must not read as paused just because that other run is
  // (`ImportRunTile` gates the same way).
  const bundlePaused = $derived(bundleRun?.status === 'running' && imports.paused)
  const queuedNotice = $derived(
    imports.latestBundleReport?.cancelled
      ? queuedDiscardedNotice(imports.latestBundleReport.queuedDiscarded)
      : null,
  )

  // Neither of these can be `true` at the same time as `bundleRun`: the pick
  // UI below is offered only while nothing is running and no report is up
  // (design D5, D4), and starting a run is the only way `bundleRun` becomes
  // non-null, which only `bundlePick.confirm()` — reachable from that same
  // UI — can do.
  const picking = $derived(!bundleRun && !imports.latestBundleReport)

  function pick() {
    void bundlePick.pick(pickBundleFiles)
  }
</script>

{#snippet outcomeGroup(label: string, group: BundleOutcomeGroup, destructive: boolean)}
  {#if group.count > 0}
    <div>
      <p class="text-sm font-medium {destructive ? 'text-destructive' : ''}">
        {label} — {group.count.toLocaleString()}
      </p>
      <ul class="mt-1 max-h-48 overflow-y-auto text-xs text-muted-foreground">
        {#each group.items.slice(0, OUTCOME_PREVIEW_LIMIT) as item, index (index)}
          <li class="break-all">{item.path}{item.reason ? ` — ${item.reason}` : ''}</li>
        {/each}
      </ul>
      {#if group.count > OUTCOME_PREVIEW_LIMIT}
        <p class="mt-1 text-xs text-muted-foreground">
          and {(group.count - OUTCOME_PREVIEW_LIMIT).toLocaleString()} more
        </p>
      {/if}
    </div>
  {/if}
{/snippet}

<div data-tauri-drag-region={windowDragRegion} class="min-h-0 flex-1 overflow-y-auto">
  <div class="mx-auto flex max-w-2xl flex-col gap-8 p-8">
    <div>
      <h1 class="text-xl font-semibold">Import a legacy bundle</h1>
      <p class="mt-2 text-sm text-muted-foreground">
        Pick the SQLite files from the old extension's "Export for app" —
        <code class="rounded-sm bg-muted px-1 py-0.5 text-xs">database.db</code>, or
        <code class="rounded-sm bg-muted px-1 py-0.5 text-xs">database-part1of18.db</code>
        and the rest. One part is a complete import on its own, so they can be moved
        over and imported a few at a time. Images already in the library are skipped;
        images trashed in the old extension come in trashed.
      </p>
    </div>

    {#if picking}
      <div>
        <Button onclick={pick} disabled={bundlePick.planning}>
          {bundlePick.planning
            ? 'Reading…'
            : bundlePick.plan ? 'Choose different files…' : 'Choose bundle files…'}
        </Button>
      </div>

      {#if bundlePick.planning}
        <!-- `bundle_plan` reads every part's row count before this shows
             anything (`import-confirm` design D2); on a slow network drive
             that outlasts a click, so the screen says so rather than looking
             stuck. -->
        <p class="text-sm text-muted-foreground">Reading the picked files…</p>
      {:else if bundlePick.plan}
        <!-- Spec `legacy-bundle-import`, "Picked bundle parts are shown and
             confirmed before anything is imported": nothing below is written
             to the library until Import is pressed. -->
        <div class="flex flex-col gap-4">
          <h2 class="text-sm font-semibold">Ready to import</h2>
          <ul class="flex flex-col gap-1 text-sm">
            {#each bundlePick.plan.parts as part (part.path)}
              <li class="flex items-center justify-between gap-4">
                <span class="truncate font-mono text-xs" title={part.path}>{part.path}</span>
                {#if part.error !== null}
                  <span class="shrink-0 text-xs text-destructive">{part.error}</span>
                {:else}
                  <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
                    {(part.rows ?? 0).toLocaleString()} rows
                  </span>
                {/if}
              </li>
            {/each}
          </ul>
          <p class="text-sm">
            Total —
            <span class="font-medium tabular-nums">{bundlePick.plan.total.toLocaleString()}</span>
          </p>
          <div class="flex gap-2">
            <Button onclick={() => bundlePick.confirm()}>Import</Button>
            <Button variant="outline" onclick={() => bundlePick.discard()}>Discard</Button>
          </div>
        </div>
      {/if}

      {#if bundlePick.error}
        <p class="text-sm text-destructive">{bundlePick.error}</p>
      {/if}
    {/if}

    {#if bundleRun}
      <div class="flex flex-col gap-2 text-sm" role="status">
        {#if bundleRun.status === 'queued'}
          <p class="text-muted-foreground">Waiting for the import ahead of it to finish…</p>
        {:else if bundlePaused}
          <!-- Says it is paused rather than looking stalled (`pending-work` spec). -->
          <p class="font-medium">Paused</p>
          {#if bundleRun.progress}
            <p class="text-muted-foreground tabular-nums">
              {bundleRun.progress.done.toLocaleString()} of
              {bundleRun.progress.total.toLocaleString()}
            </p>
          {/if}
        {:else if bundleRun.progress}
          <p class="tabular-nums">
            {bundleRun.progress.done.toLocaleString()} of
            {bundleRun.progress.total.toLocaleString()}
          </p>
          <Progress value={bundleRun.progress.done} max={bundleRun.progress.total} />
        {:else}
          <p class="text-muted-foreground">{BUNDLE_COUNTING_LABEL}</p>
        {/if}

        <div class="flex gap-2">
          {#if bundleRun.status === 'running' && !bundlePaused}
            <Button size="sm" variant="outline" onclick={() => imports.pause()}>Pause</Button>
          {:else if bundlePaused}
            <Button size="sm" variant="outline" onclick={() => imports.resume()}>Resume</Button>
          {/if}
          <!--
            Waiting here only ever removes this bundle run itself; running
            here stops it and drops the whole queue behind it — same split as
            `ImportRunTile` (`pending-work` spec, design D6).
          -->
          <Button
            size="sm"
            variant="outline"
            onclick={() => (
              bundleRun.status === 'queued' ? imports.dequeue(bundleRun.id) : imports.cancel()
            )}
          >
            Cancel
          </Button>
        </div>
      </div>
    {/if}

    {#if groups}
      <div class="flex flex-col gap-4">
        <h2 class="text-sm font-semibold">Report</h2>

        {#if imports.latestBundleReport?.cancelled}
          <!-- Not an error (`pending-work` spec "Cancelling is not failing"). -->
          <p class="text-sm text-muted-foreground">
            <span class="font-medium text-foreground">Cancelled.</span> {rerunNotice('bundle')}
          </p>
          {#if queuedNotice}
            <p class="text-sm text-muted-foreground">{queuedNotice}</p>
          {/if}
        {/if}

        {@render outcomeGroup('Failed', groups.failed, true)}
        {@render outcomeGroup('Skipped', groups.skipped, false)}

        <p class="text-sm">
          Imported —
          <span class="font-medium tabular-nums">{groups.imported.toLocaleString()}</span>
        </p>

        <!--
          Spec `legacy-bundle-import`, "A finished bundle report is cleared
          when the user is done with it": clears only this route's own
          report (design D5) — the library band's card for this run, if the
          user has not seen it yet, is untouched.
        -->
        <div>
          <Button size="sm" variant="outline" onclick={() => imports.dismissBundleReport()}>
            Done
          </Button>
        </div>
      </div>
    {/if}

    <!--
      Always shown, not only after a report (spec `legacy-bundle-import`
      "Counts shown"): the numbers and the notice are what makes the browser
      comparison possible even in a later session, after the report is gone.
      Refreshed by `Sidebar.svelte` (design D7) on a library switch and on
      every import run, so this route reads `libraryCounts` rather than
      fetching its own copy.
    -->
    {#if libraryCounts.current}
      <div class="flex flex-col gap-2 border-t border-border pt-4">
        <p class="text-sm">{bundleCountsLine(libraryCounts.current)}</p>
        <p class="text-sm text-muted-foreground">{BUNDLE_MIGRATION_NOTICE}</p>
      </div>
    {/if}

    {#if imports.error || libraryCounts.error}
      <p class="text-sm text-destructive">{imports.error ?? libraryCounts.error}</p>
    {/if}
  </div>
</div>
