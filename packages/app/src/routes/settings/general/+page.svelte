<script lang="ts">
  import type { Theme } from '@boorubox/shared'
  import {
    CLICK_ZOOM_CEILING_DEFAULT,
    CLICK_ZOOM_CEILING_MAX,
    CLICK_ZOOM_CEILING_MIN,
    CLICK_ZOOM_CEILING_STEP,
    GRID_TILE_DEFAULT,
    GRID_TILE_MAX,
    GRID_TILE_MIN,
  } from '@boorubox/shared'
  import { errorText, library, setListenerPort, settings } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Slider } from '$lib/components/ui/slider'
  import { Switch } from '$lib/components/ui/switch'

  const listener = $derived(library.status?.listener ?? null)
  const theme = $derived(settings.current?.theme ?? 'system')
  // `launch-screen` design D3: defaults the switch on while the setting is
  // still loading, matching the Rust default an old settings file reads as.
  const openLastOnLaunch = $derived(settings.current?.openLastOnLaunch ?? true)

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
  // Same shape as `tileSize`: the slider follows the drag, the setting is
  // written on release (design D4, copied from D11).
  let clickZoomCeiling = $state(
    settings.current?.clickZoomCeilingPercent ?? CLICK_ZOOM_CEILING_DEFAULT,
  )
  let error = $state<string | null>(null)

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

  async function chooseOpenLastOnLaunch(value: boolean) {
    error = null
    try {
      await settings.setOpenLastOnLaunch(value)
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

  async function commitClickZoom(percent: number) {
    error = null
    try {
      await settings.setClickZoomCeilingPercent(percent)
      clickZoomCeiling = settings.current?.clickZoomCeilingPercent ?? percent
    } catch (cause) {
      error = errorText(cause)
    }
  }
</script>

<section class="flex flex-col gap-4">
  <h2 class="text-sm font-semibold">General</h2>

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

  <div class="flex flex-col gap-2">
    <label class="text-sm text-muted-foreground" for="click-zoom">
      Click zoom — up to {clickZoomCeiling / 100}× the fit
    </label>
    <Slider
      id="click-zoom"
      type="single"
      class="max-w-sm"
      min={CLICK_ZOOM_CEILING_MIN}
      max={CLICK_ZOOM_CEILING_MAX}
      step={CLICK_ZOOM_CEILING_STEP}
      bind:value={clickZoomCeiling}
      onValueCommit={commitClickZoom}
    />
  </div>

  <!--
    `launch-screen` design D3: on by default (spec `library-folder`'s
    "reopen the last library on launch without asking"), turned off to land
    on the start screen's recent list instead. Takes effect at the next
    launch — nothing here re-opens or closes the library that is running.
  -->
  <div class="flex items-center justify-between gap-3">
    <div class="min-w-0">
      <p class="text-sm font-medium">Open the last library at launch</p>
      <p class="text-sm text-muted-foreground">Takes effect at the next launch.</p>
    </div>
    <Switch
      aria-label="Open the last library at launch"
      checked={openLastOnLaunch}
      onCheckedChange={(value) => void chooseOpenLastOnLaunch(value)}
    />
  </div>

  <h3 class="mt-2 text-sm font-semibold">Capture</h3>

  <p class="text-sm">
    {#if listener?.running}
      <span class="font-medium">Listening</span>
      on 127.0.0.1:{listener.port} for captures from the browser extension.
    {:else if listener}
      <span class="font-medium text-destructive">Not listening.</span>
      BooruBox could not bind port {listener.port}{listener.error ? `: ${listener.error}` : '.'}
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

  {#if error}
    <p class="text-sm text-destructive">{error}</p>
  {/if}
</section>
