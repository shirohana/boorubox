<script lang="ts">
  import { appUpdate, library } from '$lib/api'
  import { Button } from '$lib/components/ui/button'

  // Only the two outcomes the "About" section has to say anything extra for
  // (design D4): `available` replaces Check with an Update control of its
  // own, so there is nothing more for this line to add.
  let checkMessage = $state<string | null>(null)

  /** The explicit check (design D4): answers in all three cases. */
  async function checkForUpdate() {
    checkMessage = null
    const outcome = await appUpdate.checkNow()
    if (outcome === 'current') checkMessage = 'BooruBox is up to date.'
    else if (outcome === 'failed') checkMessage = appUpdate.lastCheckError ?? 'The check failed.'
  // 'available': the button below switches to Update, which says the rest.
  }
</script>

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
