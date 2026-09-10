<script lang="ts">
  // Where a legacy-bundle run starts and where its report is read (design D7).
  // A route of its own, not a dialog off the sidebar: the run outlives it
  // (`Imports` is the singleton queue, design D9 of the local-import change),
  // so leaving and coming back still shows the same progress or report.
  import { imports, libraryCounts, pickBundleFiles } from '$lib/api'
  import {
    type BundleOutcomeGroup,
    BUNDLE_COUNTING_LABEL,
    BUNDLE_MIGRATION_NOTICE,
    bundleCountsLine,
    groupBundleReport,
  } from '$lib/components/import/bundle-report'
  import { Button } from '$lib/components/ui/button'
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

  function pick() {
    void imports.pickBundle(pickBundleFiles)
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

<div data-tauri-drag-region class="min-h-0 flex-1 overflow-y-auto">
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

    <div>
      <Button onclick={pick} disabled={bundleRun !== null}>
        {bundleRun ? 'Importing…' : 'Choose bundle files…'}
      </Button>
    </div>

    {#if bundleRun}
      <div class="flex flex-col gap-2 text-sm" role="status">
        {#if bundleRun.status === 'queued'}
          <p class="text-muted-foreground">Waiting for the import ahead of it to finish…</p>
        {:else if bundleRun.progress}
          <p class="tabular-nums">
            {bundleRun.progress.done.toLocaleString()} of
            {bundleRun.progress.total.toLocaleString()}
          </p>
          <Progress value={bundleRun.progress.done} max={bundleRun.progress.total} />
        {:else}
          <p class="text-muted-foreground">{BUNDLE_COUNTING_LABEL}</p>
        {/if}
      </div>
    {/if}

    {#if groups}
      <div class="flex flex-col gap-4">
        <h2 class="text-sm font-semibold">Report</h2>

        {@render outcomeGroup('Failed', groups.failed, true)}
        {@render outcomeGroup('Skipped', groups.skipped, false)}

        <p class="text-sm">
          Imported —
          <span class="font-medium tabular-nums">{groups.imported.toLocaleString()}</span>
        </p>
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
