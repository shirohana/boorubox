<script lang="ts">
  import { goto } from '$app/navigation'
  import { resolve } from '$app/paths'
  import { errorText, library, pickLibrary } from '$lib/api'
  import { Button } from '$lib/components/ui/button'

  // The stored path stays stored until another folder is picked (spec
  // `library-folder`), so this screen offers no way to forget it.
  const missingPath = $derived(library.status?.missingPath ?? null)
  const listener = $derived(library.status?.listener ?? null)

  let picking = $state(false)
  let pickError = $state<string | null>(null)

  async function chooseFolder() {
    picking = true
    pickError = null
    try {
      const status = await pickLibrary()
      library.set(status)
      if (status.opened) await goto(resolve('/'))
    } catch (error) {
      pickError = errorText(error)
    } finally {
      picking = false
    }
  }
</script>

<main class="mx-auto flex min-h-screen max-w-lg flex-col justify-center gap-6 p-8">
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

  <div>
    <Button onclick={chooseFolder} disabled={picking}>
      {picking ? 'Choosing…' : 'Choose folder…'}
    </Button>
    {#if pickError}
      <p class="mt-2 text-sm text-destructive">{pickError}</p>
    {/if}
  </div>

  <!--
    FIXME: the listener state belongs on a settings screen (§5); this is the
    only screen that exists, so a user with an open library never sees a bind
    failure. Move it when settings lands.
  -->
  {#if listener && !listener.running}
    <p class="border-t border-border pt-4 text-sm text-muted-foreground">
      The capture listener is not running on port {listener.port}, so the browser extension
      cannot send captures{listener.error ? `: ${listener.error}` : ''}.
    </p>
  {/if}
</main>
