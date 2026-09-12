<script lang="ts">
  import type { RebuildReport, Theme } from '@boorubox/shared'
  import { GRID_TILE_DEFAULT, GRID_TILE_MAX, GRID_TILE_MIN } from '@boorubox/shared'
  import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down'
  import {
    appUpdate,
    errorText,
    library,
    libraryCounts,
    librarySwitch,
    notes,
    openLibrary,
    rebuild,
    setListenerPort,
    settings,
    trash,
  } from '$lib/api'
  import BooruSection from '$lib/components/booru/BooruSection.svelte'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import RebuildStatus from '$lib/components/common/RebuildStatus.svelte'
  import LibraryMenu from '$lib/components/frame/LibraryMenu.svelte'
  import RulesSection from '$lib/components/rules/RulesSection.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Kbd } from '$lib/components/ui/kbd'
  import { Slider } from '$lib/components/ui/slider'
  import { KEYBOARD_MAP } from '$lib/keyboard'
  import { windowDragRegion } from '$lib/platform'

  // Design D16: no requirement says "settings contains X, Y, Z". Each section
  // here is the screen half of a fact another capability owns — the listener
  // (`capture-ingest`), the per-source counts (`library-browse`), the library
  // path and its actions (`library-switching`), the theme (`app-frame`).

  const libraryPath = $derived(library.status?.libraryPath ?? '')
  const listener = $derived(library.status?.listener ?? null)
  const theme = $derived(settings.current?.theme ?? 'system')
  // `/settings` is unreachable with no library open (the layout's gate
  // redirects to `/start`), but the control still checks: nothing here should
  // offer to rebuild a library that is not the one this screen is describing
  // (`library-sidecars` task 3.5, "the control is absent with no library
  // open").
  const canRebuild = $derived(library.status?.opened === true)

  const themes: { value: Theme, label: string }[] = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ]

  let port = $state(String(library.status?.listener.port ?? ''))
  let applying = $state(false)
  // The slider follows the drag; the setting is written on release (design
  // D11), so this local copy is what the thumb sits on until then.
  let tileSize = $state(settings.current?.gridTileSize ?? GRID_TILE_DEFAULT)
  let error = $state<string | null>(null)
  // Only the two outcomes the "About" section has to say anything extra for
  // (design D4): `available` replaces Check with an Update control of its
  // own, so there is nothing more for this line to add.
  let checkMessage = $state<string | null>(null)
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

  async function applyPort() {
    const chosen = Number(port)
    if (!Number.isInteger(chosen) || chosen < 1 || chosen > 65535) {
      error = 'A port is a whole number between 1 and 65535.'
      return
    }
    applying = true
    error = null
    try {
      // Never rejects on a port it cannot bind: the answer is a stopped
      // listener with the reason, and the chosen port is stored either way
      // (design D6), so the field keeps showing what the user chose.
      library.setListener(await setListenerPort(chosen))
    } catch (cause) {
      error = errorText(cause)
    } finally {
      applying = false
    }
  }

  async function chooseTheme(value: Theme) {
    error = null
    try {
      await settings.setTheme(value)
    } catch (cause) {
      error = errorText(cause)
    }
  }

  async function commitTileSize(size: number) {
    error = null
    try {
      await settings.setGridTileSize(size)
      tileSize = settings.current?.gridTileSize ?? size
    } catch (cause) {
      error = errorText(cause)
    }
  }

  /** The explicit check (design D4): answers in all three cases. */
  async function checkForUpdate() {
    checkMessage = null
    const outcome = await appUpdate.checkNow()
    if (outcome === 'current') checkMessage = 'BooruBox is up to date.'
    else if (outcome === 'failed') checkMessage = appUpdate.lastCheckError ?? 'The check failed.'
  // 'available': the button below switches to Update, which says the rest.
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

<!-- No toolbar band on this screen, so its background is the drag region (D13). -->
<div data-tauri-drag-region={windowDragRegion} class="min-h-0 flex-1 overflow-y-auto">
  <div class="mx-auto flex max-w-2xl flex-col gap-10 p-8">
    <h1 class="text-xl font-semibold">Settings</h1>

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
    </section>

    <section class="flex flex-col gap-4">
      <h2 class="text-sm font-semibold">Capture</h2>

      <p class="text-sm">
        {#if listener?.running}
          <span class="font-medium">Listening</span>
          on 127.0.0.1:{listener.port} for captures from the browser extension.
        {:else if listener}
          <span class="font-medium text-destructive">Not listening.</span>
          BooruBox could not bind port {listener.port}{listener.error
            ? `: ${listener.error}`
            : '.'}
        {/if}
      </p>

      <form
        class="flex items-end gap-2"
        onsubmit={(event) => {
          event.preventDefault()
          void applyPort()
        }}
      >
        <div>
          <label class="mb-1 block text-xs text-muted-foreground" for="listener-port">Port</label>
          <Input
            id="listener-port"
            class="w-28"
            inputmode="numeric"
            bind:value={port}
            autocomplete="off"
          />
        </div>
        <Button type="submit" variant="secondary" disabled={applying}>
          {applying ? 'Applying…' : 'Apply'}
        </Button>
      </form>
    </section>

    <!--
      Slot Settings · Rules (`auto-tag-rules` design D11). Beside Capture rather
      than on a route of its own: both are about what happens as an image enters
      the library, and a nav item is for a place you look at images.
    -->
    <RulesSection />

    <!--
      Slot Settings · Booru (`booru-upload` design D13): a sibling of Rules, not
      a shared section — sites and rules are two unrelated tables of library
      configuration, and one section named after neither would hold both.
    -->
    <BooruSection />

    <section class="flex flex-col gap-4">
      <h2 class="text-sm font-semibold">Appearance</h2>

      <div class="flex flex-col gap-2">
        <p class="text-sm text-muted-foreground">Theme</p>
        <div class="flex gap-2">
          {#each themes as option (option.value)}
            <Button
              size="sm"
              variant={theme === option.value ? 'default' : 'outline'}
              aria-pressed={theme === option.value}
              onclick={() => chooseTheme(option.value)}
            >
              {option.label}
            </Button>
          {/each}
        </div>
      </div>

      <div class="flex flex-col gap-2">
        <label class="text-sm text-muted-foreground" for="tile-size">
          Default thumbnail size — {tileSize}px
        </label>
        <Slider
          id="tile-size"
          type="single"
          class="max-w-sm"
          min={GRID_TILE_MIN}
          max={GRID_TILE_MAX}
          step={10}
          bind:value={tileSize}
          onValueCommit={commitTileSize}
        />
      </div>
    </section>

    <!--
      `app-update`: the running version, read off the status payload rather
      than a command of its own (design D3's single-source rule — the string
      shown here and the one `GET /status` answers must never be able to
      drift), and a check the user can ask for on demand.
    -->
    <section class="flex flex-col gap-4">
      <h2 class="text-sm font-semibold">About</h2>

      <div class="flex flex-wrap items-center justify-between gap-3">
        <!-- Nothing to read before the first `library_status()` answer, rather
             than a label over a blank. -->
        {#if library.status}
          <p class="text-sm">
            Version <span class="font-mono">{library.status.version}</span>
          </p>
        {/if}
        {#if appUpdate.available}
          <Button size="sm" onclick={() => appUpdate.reopen()}>
            Update to {appUpdate.available?.version}
          </Button>
        {:else}
          <Button
            size="sm"
            variant="outline"
            disabled={appUpdate.checking}
            onclick={() => void checkForUpdate()}
          >
            {appUpdate.checking ? 'Checking…' : 'Check for updates'}
          </Button>
        {/if}
      </div>

      {#if checkMessage}
        <p class="text-sm text-muted-foreground">{checkMessage}</p>
      {/if}
    </section>

    <!-- The map is read-only here; every binding fires where it acts (design D14). -->
    <section class="flex flex-col gap-4">
      <h2 class="text-sm font-semibold">Keyboard</h2>
      <dl class="grid grid-cols-[auto_auto_minmax(0,1fr)] items-baseline gap-x-6 gap-y-2 text-sm">
        {#each KEYBOARD_MAP as binding (binding.where + binding.action)}
          <dt class="text-muted-foreground">{binding.where}</dt>
          <dd class="flex flex-wrap gap-1">
            {#each binding.keys as key (key)}
              <Kbd>{key}</Kbd>
            {/each}
          </dd>
          <dd>{binding.action}</dd>
        {/each}
      </dl>
    </section>

    {#if error || libraryCounts.error}
      <p class="text-sm text-destructive">{error ?? libraryCounts.error}</p>
    {/if}
  </div>
</div>

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
