<script lang="ts">
  import { goto } from '$app/navigation'
  import { resolve } from '$app/paths'
  import { page } from '$app/state'
  import {
    appUpdate,
    library,
    onCaptureStored,
    onLibraryOpened,
    pendingCaptures,
    settings,
    sidecarsBackfill,
    thumbsRegenerate,
  } from '$lib/api'
  import OpeningScreen from '$lib/components/common/OpeningScreen.svelte'
  import AppSidebar from '$lib/components/frame/Sidebar.svelte'
  import LibrarySwitchDialog from '$lib/components/frame/LibrarySwitchDialog.svelte'
  import TopBar from '$lib/components/frame/TopBar.svelte'
  import * as Sidebar from '$lib/components/ui/sidebar'
  import UpdateDialog from '$lib/components/update/UpdateDialog.svelte'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { fullscreen } from '$lib/fullscreen.svelte'
  import { KEY_FULLSCREEN, KEY_ZOOM_OUT, KEY_ZOOM_RESET, KEYS_ZOOM_IN } from '$lib/keyboard'
  import { isMacos } from '$lib/platform'
  import { applyTheme } from '$lib/theme.svelte'
  import { zoomBy } from '$lib/zoom'
  import '../app.css'

  let { children } = $props()

  const onStart = $derived(page.url.pathname === '/start')
  const libraryOpen = $derived(library.status?.opened === true)
  // `launch-screen` design D2: the folder a launch-time open is still running
  // against, read off the status the same way as `libraryOpen` rather than in
  // an `$effect` — a store reassigned wholesale on every refresh re-runs an
  // effect that reads a field off it directly (repo CLAUDE.md).
  const opening = $derived(library.status?.opening ?? null)
  const failure = $derived(library.error ?? settings.error)
  const libraryAnswered = $derived(library.status !== null || library.error !== null)
  const settingsAnswered = $derived(settings.current !== null || settings.error !== null)

  void settings.load()
  // Design D4: fire-and-forget, silent on failure — nothing here may delay
  // the window above from appearing.
  void appUpdate.checkOnLaunch()

  // Design D12: the theme is painted from the setting, and the layout renders
  // nothing until the setting has arrived, so there is no frame of the light
  // palette to see. The teardown drops the `prefers-color-scheme` listener that
  // `system` installs — without it an explicit choice would be repainted the
  // next time the OS switched appearance.
  $effect(() => applyTheme(settings.current?.theme ?? 'system'))

  // Design D2: the Rust side clears `AppState.launch_opening` and only then
  // emits `library:opened`. Reading `library_status` first and subscribing
  // afterwards risks a fast open settling, and its event firing, in the gap
  // between the two — the event would be lost and the opening screen would
  // never clear. Subscribing before the first status read closes that gap:
  // `library.load()` only fires once the subscription is registered, so any
  // event the open can still cause is one this listener is already up for.
  $effect(() => {
    const subscription = onLibraryOpened(() => void library.refresh())
    subscription.catch(() => {})
    void subscription.then(() => library.load())
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

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

  // Subscribed here and not on the library route (design D3): the route mounts
  // and unmounts, and a capture announced while the user is on another screen
  // has to be on the band when they come back — a Tauri event emitted with no
  // listener is simply lost.
  $effect(() => {
    const subscription = pendingCaptures.subscribe()
    subscription.catch(() => {})
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  // Same reasoning as the subscription above: a library's catch-up pass can
  // finish while the user is on another screen, and the pending-work band
  // reads `sidecarsBackfill.progress` on whichever route shows it.
  $effect(() => {
    const subscription = sidecarsBackfill.subscribe()
    subscription.catch(() => {})
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  // Same reasoning again: a thumbnail regeneration is started from Settings,
  // but its last tick has to drop the thumbnail cache (design D6) even if the
  // user has since left that screen.
  $effect(() => {
    const subscription = thumbsRegenerate.subscribe()
    subscription.catch(() => {})
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  // The gate from spec `library-folder`: with no library open, `/start` is the
  // only reachable route. The children stay unrendered until the redirect has
  // landed — rendering them first flashes the grid, search and import. Not
  // while `opening` is set: a launch-time open still running is not "no
  // library open", and redirecting here would fight the opening screen below.
  $effect(() => {
    if (library.status && !opening && !libraryOpen && !onStart) void goto(resolve('/start'))
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

  // F11 on every platform (design D2). Not guarded by the typing check either:
  // F11 types nothing. `preventDefault` so the webview's own full-screen
  // handling of the key does nothing on top of this.
  function fullscreenKey(event: KeyboardEvent) {
    if (event.key !== KEY_FULLSCREEN) return
    event.preventDefault()
    void fullscreen.toggle()
  }

  function windowKeys(event: KeyboardEvent) {
    zoomKeys(event)
    fullscreenKey(event)
  }

  /**
   * Leaving macOS fullscreen (⌃⌘F) leaves the window itself as first responder:
   * the webview is no longer where keys go, and nothing on the page fires until
   * a click — not the grid's arrows, not even `/` bound on the window. WebKit
   * reports the loss as a `blur` on the window. When the app window still has
   * the focus, the loss happened inside it, so the webview asks for the keys
   * back (`core:webview:allow-set-webview-focus`). A blur from switching apps
   * leaves `isFocused` false and is left alone.
   *
   * macOS only, and not because Windows has no fullscreen to leave: on Windows
   * a drag by the native title bar blurs the webview while the window stays
   * focused (tauri-apps/tauri#10767, the native frame included), so this
   * handler pulled the focus back mid-drag, the drag blurred it again, and the
   * app looped focus/blur — slow and deaf to clicks — until a switch to another
   * window turned `isFocused` false. Bound as `undefined` there, not guarded
   * inside, so the listener is never even installed.
   */
  async function reclaimKeys() {
    if (!(await getCurrentWindow().isFocused())) return
    await getCurrentWebview().setFocus()
  }
</script>

<svelte:window
  onkeydown={windowKeys}
  onblur={isMacos ? reclaimKeys : undefined}
  onresize={() => void fullscreen.refresh()}
/>

<!--
  Mounted unconditionally, not only once the library is open: a launch check
  can find an update before `library.load()` even answers, and the dialog
  gates itself on `appUpdate.promptOpen`, so there is nothing to render while
  it has none.
-->
<UpdateDialog />
<LibrarySwitchDialog />

{#if failure}
  <main class="p-6">
    <h1 class="text-lg font-semibold">BooruBox could not read its settings</h1>
    <p class="mt-2 text-sm text-muted-foreground">{failure}</p>
  </main>
{:else if !libraryAnswered || !settingsAnswered}
  <OpeningScreen />
{:else if opening}
  <!--
    Checked ahead of `onStart`/`libraryOpen` (design D2): `opened` can read
    true for one poll right at the end of the launch-time open, before
    `opening` clears, but landing on the library UI either way is fine — the
    next `library.refresh()` clears `opening` and this branch falls through.
  -->
  <OpeningScreen path={opening} />
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
