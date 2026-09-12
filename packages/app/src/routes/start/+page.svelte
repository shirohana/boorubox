<script lang="ts">
  import type { LibraryStatus, RecentLibrary } from '@boorubox/shared'
  import { goto } from '$app/navigation'
  import { resolve } from '$app/paths'
  import {
    appUpdate,
    errorText,
    forgetRecent,
    library,
    librarySwitch,
    openLibrary,
    pickLibrary,
    rebuild,
    recentLibraries,
  } from '$lib/api'
  import RebuildStatus from '$lib/components/common/RebuildStatus.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { windowDragRegion } from '$lib/platform'

  // The stored path stays stored until another folder is picked (spec
  // `library-folder`), so this screen offers no way to forget it.
  const missingPath = $derived(library.status?.missingPath ?? null)
  // Distinct from `missingPath` (`library-sidecars` design D9): a damaged
  // library sits right where it always did, so "missing" would be wrong and
  // frightening for it. `status` never sets both, but the wording below
  // still branches on this first so the two can never show at once.
  const damagedPath = $derived(library.status?.damagedPath ?? null)
  // The third reason a remembered library will not open (`library-recovery`,
  // "A library from a newer build"): not missing, not damaged, and never
  // offered a rebuild — rebuilding it would silently downgrade it. Without a
  // branch of its own this screen would fall through to "Choose a library
  // folder" and never name the folder the user already has.
  const newerPath = $derived(library.status?.newerPath ?? null)
  const listener = $derived(library.status?.listener ?? null)
  const rebuildReport = $derived(rebuild.report)

  let recent = $state<RecentLibrary[]>([])
  let busy = $state(false)
  let error = $state<string | null>(null)
  // Only the two outcomes the check has to say something extra for, exactly as
  // on /settings: 'available' opens the layout's dialog, which says the rest.
  let checkMessage = $state<string | null>(null)

  // A rebuild moves the library's database aside and renames a fresh one onto
  // it (design D10, D11), so every other door into a library stays shut while
  // one runs: opening the folder being rebuilt — or any folder, since the
  // rebuild holds Rust — is the second writer this whole change exists to
  // keep out.
  const blocked = $derived(busy || rebuild.running)

  // Design D4: `available` is a `stat` per entry, computed when the list is
  // asked for. Asking here rather than at startup is the point — ten entries on
  // an unmounted volume would otherwise hold the window back from appearing.
  recentLibraries()
    .then((entries) => (recent = entries))
    .catch((cause) => (error = errorText(cause)))

  async function enter(open: () => Promise<LibraryStatus>) {
    if (blocked) return
    busy = true
    error = null
    try {
      // Wired through the same guard every swap path uses (design D6), even
      // though nothing here ever finds a question to ask: closing a library
      // already cancels its import before the redirect that lands on this
      // screen. Leaving this door unwired is how the next one gets built
      // without one.
      await librarySwitch.guard('switch', async () => {
        const status = await open()
        library.set(status)
        if (status.opened) await goto(resolve('/'))
      })
    } catch (cause) {
      error = errorText(cause)
    } finally {
      busy = false
    }
  }

  async function forget(path: string) {
    error = null
    try {
      recent = await forgetRecent(path)
    } catch (cause) {
      error = errorText(cause)
    }
  }

  /**
   * The damaged state's Rebuild action (`library-sidecars` design D12): asked
   * for, never automatic. Leaves `rebuild.report` set so the panel shows what
   * happened before offering to open the library.
   */
  async function runRebuild() {
    if (!damagedPath || rebuild.running) return
    await rebuild.run(damagedPath)
  }

  /**
   * The report's own Open button. Goes through {@link enter}, the same guard
   * and redirect every other way into the library uses, then clears the
   * report so a second damaged folder starts clean — but only once the
   * library is actually open. An open that failed leaves the report standing:
   * the kept-aside name is in it, it cannot be asked for again, and dropping
   * it would offer the user a second rebuild of a library that has already
   * been rebuilt.
   */
  async function openRebuilt() {
    if (!damagedPath) return
    await enter(() => openLibrary(damagedPath))
    if (!error) rebuild.reset()
  }

  /**
   * The newer-library state's one action: the update this build needs is the
   * only way into that folder. `appUpdate` is a singleton and `UpdateDialog`
   * is mounted by the root layout on every route including this one, so the
   * same check Settings offers is reachable here with nothing new wired —
   * which matters, because Settings is behind the library that will not open.
   */
  async function checkForUpdate() {
    checkMessage = null
    const outcome = await appUpdate.checkNow()
    // 'available' says nothing here: the layout's dialog is already asking
    // about it, which is the whole answer this screen needed.
    if (outcome === 'current') {
      checkMessage = 'BooruBox is up to date. This library needs a build that is not out yet.'
    } else if (outcome === 'failed') {
      checkMessage = appUpdate.lastCheckError ?? 'The check failed.'
    }
  }
</script>

<!-- No frame here, so the window's background is the drag region (D13). -->
<main
  data-tauri-drag-region={windowDragRegion}
  class="mx-auto flex min-h-svh max-w-lg flex-col justify-center gap-6 p-8"
>
  <div>
    <h1 class="text-2xl font-semibold">
      {#if damagedPath}
        This library's index is damaged
      {:else if newerPath}
        This library needs a newer BooruBox
      {:else if missingPath}
        Your library folder is missing
      {:else}
        Choose a library folder
      {/if}
    </h1>
    <p class="mt-2 text-sm text-muted-foreground">
      {#if damagedPath}
        BooruBox could not open <span class="font-mono break-all">{damagedPath}</span> because its
        index is damaged. Rebuilding reads every image's own file to build a fresh one; the
        current database is kept, never deleted.
      {:else if newerPath}
        <span class="font-mono break-all">{newerPath}</span> was written by a newer version of
        BooruBox, so this one cannot open it. Update BooruBox and open it again; nothing in the
        folder has been changed, and rebuilding it is not the answer — it would downgrade the
        library.
      {:else if missingPath}
        BooruBox could not open <span class="font-mono break-all">{missingPath}</span>. Reconnect
        that drive or folder and restart, or choose another folder to use instead.
      {:else}
        BooruBox keeps every image and all of its metadata inside one folder, so you can back it
        up or move it by copying the folder.
      {/if}
    </p>
  </div>

  {#if newerPath}
    <!--
      No Rebuild here (spec `library-recovery`: "A rebuild SHALL NOT be offered
      for a library refused for any other reason, in particular one written by
      a newer version of the app"). The update is the way in, so the check is
      offered where the refusal is met rather than pointing at a Settings
      screen that is behind the library this folder would have opened.
    -->
    <div class="flex flex-col gap-3 rounded-lg border border-border p-4">
      <Button onclick={() => checkForUpdate()} disabled={appUpdate.checking}>
        {appUpdate.checking ? 'Checking…' : 'Check for updates'}
      </Button>
      {#if checkMessage}
        <p class="text-sm text-muted-foreground">{checkMessage}</p>
      {/if}
    </div>
  {/if}

  {#if damagedPath}
    <!--
      The damaged state's own flow (design D12): Rebuild, progress, then the
      report before the Open button — the user reads what happened before the
      grid replaces it (spec `library-recovery`, "the report is shown before
      the library opens").
    -->
    <div class="flex flex-col gap-3 rounded-lg border border-border p-4">
      <RebuildStatus
        running={rebuild.running}
        progress={rebuild.progress}
        report={rebuildReport}
      />
      {#if rebuildReport}
        <Button onclick={() => openRebuilt()} disabled={busy}>
          {busy ? 'Opening…' : 'Open library'}
        </Button>
      {:else if !rebuild.running}
        <Button onclick={() => runRebuild()}>Rebuild library index</Button>
      {/if}
      {#if rebuild.error}
        <p class="text-sm text-destructive">{rebuild.error}</p>
      {/if}
    </div>
  {/if}

  <!-- Nothing when the list is empty: no heading over no rows, no placeholder
       row (spec `library-switching`, "Nothing opened yet"). -->
  {#if recent.length > 0}
    <ul class="flex flex-col gap-1">
      {#each recent as entry (entry.path)}
        <li class="flex items-center gap-2">
          {#if entry.available}
            <button
              type="button"
              class="
                min-w-0 flex-1 rounded-lg px-3 py-2 text-left
                hover:bg-accent
                focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-hidden
              "
              disabled={blocked}
              onclick={() => enter(() => openLibrary(entry.path))}
            >
              <span class="block truncate text-sm font-medium">{entry.name}</span>
              <span class="block truncate font-mono text-xs text-muted-foreground">
                {entry.path}
              </span>
            </button>
          {:else}
            <!--
              FIXME: an unavailable entry is not clickable because `open_library`
              creates a library in whatever folder it is given, and spec
              `library-switching` says a folder that has gone SHALL be reported,
              not recreated. Disabling the row only covers the entries this list
              already knew about: a folder that vanishes between the list and the
              click would still be recreated. The right shape is an
              `open_library` that refuses to create — a mode, or a separate
              `open_existing_library` — and then this row can be clickable and
              report what came back.
            -->
            <div class="min-w-0 flex-1 px-3 py-2">
              <span class="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                <span class="truncate">{entry.name}</span>
                <Badge variant="outline">Unavailable</Badge>
              </span>
              <span class="block truncate font-mono text-xs text-muted-foreground">
                {entry.path}
              </span>
            </div>
          {/if}
          <Button
            size="sm"
            variant="ghost"
            disabled={blocked}
            onclick={() => forget(entry.path)}
          >
            Forget
          </Button>
        </li>
      {/each}
    </ul>
  {/if}

  <div>
    <Button onclick={() => enter(pickLibrary)} disabled={blocked}>
      {busy ? 'Opening…' : 'Choose folder…'}
    </Button>
    {#if error}
      <p class="mt-2 text-sm text-destructive">{error}</p>
    {/if}
  </div>

  <!--
    Kept here as well as on /settings: with no library open the frame — and so
    the settings screen — is out of reach (design D2), and this is the one
    screen a user with a taken port will be looking at.
  -->
  {#if listener && !listener.running}
    <p class="border-t border-border pt-4 text-sm text-muted-foreground">
      The capture listener is not running on port {listener.port}, so the browser extension
      cannot send captures{listener.error ? `: ${listener.error}` : ''}.
    </p>
  {/if}
</main>
