<script lang="ts">
  // Test support only, for `Inspector.test-harness.svelte`: a genuine child
  // of `Tooltip.Provider` (context is tree-shaped, not lexical — this has to
  // be a real descendant), so `getAllContexts()` here — called during this
  // component's own initialisation, same rule as `getContext` — returns the
  // provider's context and nothing else does.
  import { getAllContexts, untrack } from 'svelte'

  interface Props {
    oncaptured: (context: Map<unknown, unknown>) => void
  }

  let { oncaptured }: Props = $props()

  // One shot, at mount — this component is never updated, so the usual
  // staleness `untrack` warns about does not apply.
  untrack(() => oncaptured(getAllContexts()))
</script>
