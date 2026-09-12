<script lang="ts">
  // The one dialog spec `library-switching` puts in front of a library swap
  // while an import is running or waiting (`import-confirm` design D6).
  // Mounted unconditionally in the root layout, the way `UpdateDialog` is
  // (both are reachable from `/start`, which renders outside the frame):
  // `librarySwitch.guard` is what every swap path already calls, so this
  // component only ever renders the question that call opened.
  import { librarySwitch } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { switchConfirmation } from '$lib/components/import/cancelled-import'

  // `librarySwitch.pending` is reassigned wholesale (a new object, or `null`)
  // rather than mutated in place, so its fields are read inside `$derived`
  // here, never off an `$effect` (CLAUDE.md).
  const question = $derived(
    librarySwitch.pending
      ? switchConfirmation(
        librarySwitch.pending.kind,
        librarySwitch.pending.queued,
        librarySwitch.pending.action,
      )
      : null,
  )
</script>

{#if question}
  <ConfirmDialog
    title={question.title}
    description={librarySwitch.stopping ? 'Stopping the import…' : question.description}
    confirmLabel={librarySwitch.stopping ? 'Stopping…' : question.confirmLabel}
    open
    onclose={() => librarySwitch.decline()}
    onconfirm={() => librarySwitch.confirm()}
  />
{/if}
