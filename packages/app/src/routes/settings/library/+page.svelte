<script lang="ts">
  import type { RebuildReport } from '@boorubox/shared'
  import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down'
  import {
    errorText,
    library,
    libraryCounts,
    librarySwitch,
    notes,
    openLibrary,
    rebuild,
    thumbsRegenerate,
    trash,
  } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import RebuildStatus from '$lib/components/common/RebuildStatus.svelte'
  import LibraryMenu from '$lib/components/frame/LibraryMenu.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Progress } from '$lib/components/ui/progress'

  const libraryPath = $derived(library.status?.libraryPath ?? '')
  // `/settings` is unreachable with no library open (the layout's gate
  // redirects to `/start`), but the control still checks: nothing here should
  // offer to rebuild a library that is not the one this screen is describing
  // (`library-sidecars` task 3.5, "the control is absent with no library
  // open").
  const canRebuild = $derived(library.status?.opened === true)

  let error = $state<string | null>(null)
  let rebuildConfirmOpen = $state(false)
  // This screen's own copy of the last rebuild's report: `rebuild.report` is
  // cleared as soon as the flow ends, so that a folder met as damaged later
  // never opens the start screen on a report of some other rebuild.
  let rebuildResult = $state<RebuildReport | null>(null)

  // `libraryCounts` (`legacy-bundle-import` design D7) is refreshed by
  // `Sidebar.svelte` on a library switch and on every import run, for both
  // this screen and `/import` — reading it here, this screen needs no trigger
  // of its own.
  const countRows = $derived(libraryCounts.current
    ? [
      { label: 'Total', value: libraryCounts.current.total },
      { label: 'Extension', value: libraryCounts.current.extension },
      { label: 'Local', value: libraryCounts.current.local },
      { label: 'Legacy bundle', value: libraryCounts.current.legacyBundle },
    ]
    : [])

  /**
   * Starts a regeneration pass (`one-level-buckets` design D4, D5). The store
   * keeps its own `error`, read back here only on a refusal — `Busy` while a
   * pass already runs, which the disabled button otherwise prevents.
   */
  async function runRegenerateThumbnails() {
    error = null
    const outcome = await thumbsRegenerate.start()
    if (!outcome) error = thumbsRegenerate.error
  }

  /**
   * Confirmed rebuild of the open library's own index (`library-sidecars`
   * design D12, spec `library-recovery` "Rebuilding a library that opens").
   * `rebuild_library` closes the library first, so the note is flushed before
   * it goes — the same reasoning `LibraryMenu`'s close and switch actions
   * follow (`notes` design D14).
   *
   * The reopen is not conditioned on the rebuild having worked: the command
   * leaves nothing open either way (design D12), so returning early on a
   * failure would leave this frame — the sidebar, the counts, the grid behind
   * it — describing a library Rust has closed, with every command behind it
   * answering that none is open. A reopen that fails records why in Rust
   * (design D9), so the refreshed status sends the layout's gate to /start,
   * where the damaged state offers the rebuild again.
   */
  async function confirmRebuild() {
    rebuildConfirmOpen = false
    const path = library.status?.libraryPath
    if (!path) return
    // A rebuild closes the library first, so it is a swap path like any other
    // (`library-switching` spec): with an import running it asks before it
    // cancels, through the one guard every such path goes through.
    await librarySwitch.guard('close', async () => {
      error = null
      rebuildResult = null
      await notes.flush()
      const report = await rebuild.run(path)
      rebuildResult = report
      error = report ? null : rebuild.error
      try {
        library.set(await openLibrary(path))
      } catch (cause) {
        // The rebuild's own failure is the one worth reading when there are
        // two: the reopen failed because of it.
        error ??= errorText(cause)
        await library.refresh()
      }
      // The store is the start screen's — it reads a report as the outcome of
      // the rebuild it asked for — and this screen has taken its own copy
      // above, so nothing is left behind for a later damaged folder to show.
      rebuild.reset()
    })
  }
</script>

<section class="flex flex-col gap-4">
  <h2 class="text-sm font-semibold">Library</h2>

  <div class="flex flex-wrap items-center justify-between gap-3">
    <p class="min-w-0 flex-1 font-mono text-sm break-all">{libraryPath}</p>
    <!--
      The same menu the sidebar footer opens — switching, choosing a folder,
      revealing it and closing are one set of actions, written once.
    -->
    <LibraryMenu align="end" side="bottom" onerror={(message) => (error = message)}>
      {#snippet trigger({ props })}
        <Button variant="outline" size="sm" {...props}>
          Manage library
          <ChevronsUpDownIcon />
        </Button>
      {/snippet}
    </LibraryMenu>
  </div>

  <!--
    These counts are the whole library, never the current search: they are
    what makes a migration verifiable (spec `library-browse`), which is why
    they live here rather than over the grid (design D7).
  -->
  <dl class="grid grid-cols-2 gap-x-6 gap-y-1 text-sm sm:grid-cols-4">
    {#each countRows as row (row.label)}
      <div class="flex items-baseline justify-between gap-2 border-t border-border pt-1">
        <dt class="text-muted-foreground">{row.label}</dt>
        <dd class="font-medium tabular-nums">{row.value.toLocaleString()}</dd>
      </div>
    {/each}
  </dl>

  <!--
    `trash` design D9: the counts above exclude trashed images on purpose —
    they are what §9 step 4 compares against a browser viewer whose own
    count excludes its trash. What the app holds is still answered, beside
    them rather than folded into them. The sidebar's badge is the same
    number, from the same store.
  -->
  <p class="text-sm text-muted-foreground">
    In trash —
    <span class="font-medium text-foreground tabular-nums">
      {trash.count.toLocaleString()}
    </span>
    {trash.count === 1 ? 'image' : 'images'}, not counted above.
  </p>

  <!--
    `one-level-buckets` design D4, D5: a user-started background pass over
    every thumbnail, off the lock and per image, so the library stays
    usable while it runs. Always offered, unlike Rebuild below: it needs
    no confirm, since nothing it does is destructive.
  -->
  <div class="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
    <div class="min-w-0">
      <p class="text-sm font-medium">Regenerate thumbnails</p>
      <p class="text-sm text-muted-foreground">
        Re-renders every thumbnail at the current size. Existing files are replaced as the
        pass reaches them.
      </p>
    </div>
    <Button
      variant="outline"
      size="sm"
      disabled={thumbsRegenerate.running}
      onclick={() => void runRegenerateThumbnails()}
    >
      {thumbsRegenerate.running ? 'Regenerating…' : 'Regenerate thumbnails'}
    </Button>
  </div>

  <!--
    The report and the error are read from the store directly, not the
    page's own `error` (which `runRegenerateThumbnails` also sets, for a
    refusal caught while this screen is mounted): the store keeps both
    for the window's life, so a pass that failed or finished while the
    user was on another screen still shows when they come back. Gated on
    `report`/`error`/`progress` rather than `running`, so no empty div
    sits on screen for the instant after `start()` before the first tick
    arrives.
  -->
  {#if thumbsRegenerate.report || thumbsRegenerate.error || thumbsRegenerate.progress}
    <div class="flex flex-col gap-2">
      {#if thumbsRegenerate.report}
        <p class="text-sm text-muted-foreground tabular-nums">
          Regenerated {thumbsRegenerate.report.regenerated.toLocaleString()}
          {#if thumbsRegenerate.report.failed > 0}
            — {thumbsRegenerate.report.failed.toLocaleString()} could not be read
          {/if}
        </p>
      {:else if thumbsRegenerate.error}
        <p class="text-sm text-destructive">{thumbsRegenerate.error}</p>
      {:else if thumbsRegenerate.progress}
        <p class="text-sm text-muted-foreground tabular-nums">
          {thumbsRegenerate.progress.done.toLocaleString()} of
          {thumbsRegenerate.progress.total.toLocaleString()}
        </p>
        <Progress
          value={thumbsRegenerate.progress.done}
          max={thumbsRegenerate.progress.total}
        />
      {/if}
    </div>
  {/if}

  {#if canRebuild}
    <!--
      `library-sidecars` design D12, spec `library-recovery`: manual, not
      only for damage — a database can be stale rather than damaged (a
      sync client resurrecting yesterday's file passes `quick_check`
      perfectly), and this is the only way back for one.
    -->
    <div class="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
      <div class="min-w-0">
        <p class="text-sm font-medium">Rebuild library index</p>
        <p class="text-sm text-muted-foreground">
          Rebuilds the database from every image's own file. The current database is kept
          aside, never deleted.
        </p>
      </div>
      <Button
        variant="outline"
        size="sm"
        disabled={rebuild.running}
        onclick={() => (rebuildConfirmOpen = true)}
      >
        {rebuild.running ? 'Rebuilding…' : 'Rebuild library index'}
      </Button>
    </div>

  {/if}

  <!--
    The same progress and the same report the start screen shows (spec
    `library-recovery`): a 25,000-image rebuild behind a disabled button
    alone is the screen that "appears stalled", and the name the old
    database was kept under is only ever said once — here.

    Outside `canRebuild` on purpose: a rebuild that worked but whose reopen
    did not leaves no library open, so gating the report on one would
    unmount it at exactly the moment it is worth reading — and the
    kept-aside name it carries cannot be asked for again. The local
    `rebuildResult` is this screen's own copy and outlives the store's,
    which `confirmRebuild` clears.
  -->
  {#if rebuild.running || rebuildResult}
    <div class="flex flex-col gap-2">
      <RebuildStatus
        running={rebuild.running}
        progress={rebuild.progress}
        report={rebuildResult}
      />
    </div>
  {/if}

  {#if error || libraryCounts.error}
    <p class="text-sm text-destructive">{error || libraryCounts.error}</p>
  {/if}
</section>

<ConfirmDialog
  title="Rebuild the library index?"
  description="The current database is kept aside, never deleted, under a name the result names.
    This can take a while for a large library."
  confirmLabel="Rebuild"
  destructive={false}
  open={rebuildConfirmOpen}
  onclose={() => (rebuildConfirmOpen = false)}
  onconfirm={() => void confirmRebuild()}
/>
