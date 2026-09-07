<script lang="ts">
  import { goto } from '$app/navigation'
  import { resolve } from '$app/paths'
  import { page } from '$app/state'
  import { library, onCaptureStored, settings } from '$lib/api'
  import AppSidebar from '$lib/components/frame/Sidebar.svelte'
  import TopBar from '$lib/components/frame/TopBar.svelte'
  import * as Sidebar from '$lib/components/ui/sidebar'
  import { KEY_ZOOM_OUT, KEY_ZOOM_RESET, KEYS_ZOOM_IN } from '$lib/keyboard'
  import { applyTheme } from '$lib/theme.svelte'
  import { zoomBy } from '$lib/zoom'
  import '../app.css'

  let { children } = $props()

  const onStart = $derived(page.url.pathname === '/start')
  const libraryOpen = $derived(library.status?.opened === true)
  const failure = $derived(library.error ?? settings.error)
  const libraryAnswered = $derived(library.status !== null || library.error !== null)
  const settingsAnswered = $derived(settings.current !== null || settings.error !== null)

  void library.load()
  void settings.load()

  // Design D12: the theme is painted from the setting, and the layout renders
  // nothing until the setting has arrived, so there is no frame of the light
  // palette to see. The teardown drops the `prefers-color-scheme` listener that
  // `system` installs — without it an explicit choice would be repainted the
  // next time the OS switched appearance.
  $effect(() => applyTheme(settings.current?.theme ?? 'system'))

  // The image count in the sidebar is part of the frame, so it follows a
  // capture on every screen — not only on the one that lists the images. The
  // library route has its own subscription for the grid.
  $effect(() => {
    const subscription = onCaptureStored(() => void library.refresh())
    subscription.catch(() => {})
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  // The gate from spec `library-folder`: with no library open, `/start` is the
  // only reachable route. The children stay unrendered until the redirect has
  // landed — rendering them first flashes the grid, search and import.
  $effect(() => {
    if (library.status && !libraryOpen && !onStart) void goto(resolve('/start'))
  })

  // Whole-app zoom, on every screen including /start. Not guarded by the
  // typing check: with the command key held nothing is being typed.
  function zoomKeys(event: KeyboardEvent) {
    if (!(event.metaKey || event.ctrlKey) || event.altKey) return
    const direction = KEYS_ZOOM_IN.includes(event.key)
      ? 1
      : event.key === KEY_ZOOM_OUT ? -1 : event.key === KEY_ZOOM_RESET ? 0 : null
    if (direction === null) return
    event.preventDefault()
    void zoomBy(direction)
  }
</script>

<svelte:window onkeydown={zoomKeys} />

{#if failure}
  <main class="p-6">
    <h1 class="text-lg font-semibold">BooruBox could not read its settings</h1>
    <p class="mt-2 text-sm text-muted-foreground">{failure}</p>
  </main>
{:else if !libraryAnswered || !settingsAnswered}
  <p class="p-6 text-sm text-muted-foreground">Opening your library…</p>
{:else if onStart}
  <!-- Spec `app-frame`: with no library open there is nothing for the frame to
       be about, so the start screen has the whole window. -->
  {@render children()}
{:else if libraryOpen}
  <!--
    The frame is here and not in the pages, so moving between screens keeps the
    sidebar mounted and its state (spec `app-frame`). The top bar spans the
    window above both columns; a route fills it through `frame.toolbar`.
  -->
  <Sidebar.Provider class="h-svh min-h-0! flex-col">
    <TopBar />
    <div class="flex min-h-0 flex-1">
      <AppSidebar />
      <Sidebar.Inset class="flex min-h-0 min-w-0 flex-1 flex-col">
        {@render children()}
      </Sidebar.Inset>
    </div>
  </Sidebar.Provider>
{/if}
