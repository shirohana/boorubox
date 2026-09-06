<script lang="ts">
  // Design D17: Phase 1 has no settings screen, so the listener failure the
  // `capture-ingest` spec wants surfaced lives here, where a user waiting for
  // captures will be. Move it to settings when that screen exists.
  import type { ListenerStatus } from '@boorubox/shared'

  let { listener }: { listener: ListenerStatus | null } = $props()
</script>

{#if listener && !listener.running}
  <p
    role="status"
    class="border-b border-destructive/30 bg-destructive/10 px-4 py-2 text-sm text-foreground"
  >
    <span class="font-medium text-destructive">Captures cannot arrive.</span>
    BooruBox is not listening on port {listener.port}{listener.error
      ? `: ${listener.error}`
      : '.'}
  </p>
{/if}
