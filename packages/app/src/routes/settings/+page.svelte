<script lang="ts">
  import type { Theme } from '@boorubox/shared'
  import { GRID_TILE_DEFAULT, GRID_TILE_MAX, GRID_TILE_MIN } from '@boorubox/shared'
  import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down'
  import { errorText, library, libraryCounts, setListenerPort, settings, trash } from '$lib/api'
  import BooruSection from '$lib/components/booru/BooruSection.svelte'
  import LibraryMenu from '$lib/components/frame/LibraryMenu.svelte'
  import RulesSection from '$lib/components/rules/RulesSection.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Kbd } from '$lib/components/ui/kbd'
  import { Slider } from '$lib/components/ui/slider'
  import { KEYBOARD_MAP } from '$lib/keyboard'

  // Design D16: no requirement says "settings contains X, Y, Z". Each section
  // here is the screen half of a fact another capability owns — the listener
  // (`capture-ingest`), the per-source counts (`library-browse`), the library
  // path and its actions (`library-switching`), the theme (`app-frame`).

  const libraryPath = $derived(library.status?.libraryPath ?? '')
  const listener = $derived(library.status?.listener ?? null)
  const theme = $derived(settings.current?.theme ?? 'system')

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
</script>

<!-- No toolbar band on this screen, so its background is the drag region (D13). -->
<div data-tauri-drag-region class="min-h-0 flex-1 overflow-y-auto">
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
