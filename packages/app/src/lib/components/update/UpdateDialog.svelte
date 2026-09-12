<script lang="ts">
  // The one confirmation `app-update` shows: what version is on offer, and
  // install-or-decline (spec `app-update`, "An update installs only after
  // the user agrees"). Mounted once from the root layout — like
  // `pendingCaptures`, an update can be found while the user is on any
  // screen, and the confirmation has to survive a navigation between them —
  // so this reads `appUpdate` directly rather than taking it as a prop.
  import { appUpdate, imports, library, pendingCaptures } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { workInFlight } from './update-work'

  // Design D6: read off `imports.runs` and `pendingCaptures.entries`, both
  // reassigned wholesale on every change — `$derived`, never an `$effect`
  // reading their `.length` into a local `$state`, which would go stale the
  // instant a run started or finished behind this dialog's back.
  const busyWith = $derived(workInFlight(imports.runs.length, pendingCaptures.entries.length))

  function close() {
    // Dismissing by the overlay or Esc declines — the safe direction, same
    // as `ConfirmDialog`. Not while an install is actually running: it
    // carries on in the plugin either way, and closing here would only hide
    // its outcome.
    if (!appUpdate.installing) appUpdate.decline()
  }
</script>

<Dialog.Root open={appUpdate.promptOpen} onOpenChange={(next) => { if (!next) close() }}>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title>BooruBox {appUpdate.available?.version} is available</Dialog.Title>
      <Dialog.Description>
        You're on {library.status?.version}. Installing downloads the update, verifies it, and
        restarts BooruBox on the new version.
      </Dialog.Description>
    </Dialog.Header>

    {#if appUpdate.installing}
      <p class="text-sm text-muted-foreground">Downloading and installing…</p>
    {:else if busyWith}
      <!-- Spec `app-update`, "An update is not taken while work is in flight":
           shown before a click, and it is also what `installError` repeats
           if they click anyway — a real refusal, not a delay-and-hope. -->
      <p class="text-sm text-muted-foreground">
        {busyWith[0]?.toUpperCase()}{busyWith.slice(1)}. Install once it's finished — nothing
        here interrupts it.
      </p>
    {/if}

    {#if appUpdate.installError}
      <p class="text-sm text-destructive">{appUpdate.installError}</p>
    {/if}

    <Dialog.Footer>
      <Button variant="ghost" disabled={appUpdate.installing} onclick={() => appUpdate.decline()}>
        Not now
      </Button>
      <Button
        disabled={appUpdate.installing}
        onclick={() => void appUpdate.install(() => busyWith)}
      >
        {appUpdate.installing ? 'Installing…' : 'Install and restart'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
