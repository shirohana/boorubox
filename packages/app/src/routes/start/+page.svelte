<script lang="ts">
  import type { LibraryStatus, RecentLibrary } from '@boorubox/shared'
  import { goto } from '$app/navigation'
  import { resolve } from '$app/paths'
  import {
    errorText,
    forgetRecent,
    library,
    openLibrary,
    pickLibrary,
    recentLibraries,
  } from '$lib/api'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { windowDragRegion } from '$lib/platform'

  // The stored path stays stored until another folder is picked (spec
  // `library-folder`), so this screen offers no way to forget it.
  const missingPath = $derived(library.status?.missingPath ?? null)
  const listener = $derived(library.status?.listener ?? null)

  let recent = $state<RecentLibrary[]>([])
  let busy = $state(false)
  let error = $state<string | null>(null)

  // Design D4: `available` is a `stat` per entry, computed when the list is
  // asked for. Asking here rather than at startup is the point — ten entries on
  // an unmounted volume would otherwise hold the window back from appearing.
  recentLibraries()
    .then((entries) => (recent = entries))
    .catch((cause) => (error = errorText(cause)))

  async function enter(open: () => Promise<LibraryStatus>) {
    if (busy) return
    busy = true
    error = null
    try {
      const status = await open()
      library.set(status)
      if (status.opened) await goto(resolve('/'))
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
</script>

<!-- No frame here, so the window's background is the drag region (D13). -->
<main
  data-tauri-drag-region={windowDragRegion}
  class="mx-auto flex min-h-svh max-w-lg flex-col justify-center gap-6 p-8"
>
  <div>
    <h1 class="text-2xl font-semibold">
      {missingPath ? 'Your library folder is missing' : 'Choose a library folder'}
    </h1>
    <p class="mt-2 text-sm text-muted-foreground">
      {#if missingPath}
        BooruBox could not open <span class="font-mono break-all">{missingPath}</span>. Reconnect
        that drive or folder and restart, or choose another folder to use instead.
      {:else}
        BooruBox keeps every image and all of its metadata inside one folder, so you can back it
        up or move it by copying the folder.
      {/if}
    </p>
  </div>

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
              disabled={busy}
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
          <Button size="sm" variant="ghost" onclick={() => forget(entry.path)}>Forget</Button>
        </li>
      {/each}
    </ul>
  {/if}

  <div>
    <Button onclick={() => enter(pickLibrary)} disabled={busy}>
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
