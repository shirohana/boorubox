<script lang="ts">
  // Test support only, for `Inspector.svelte.test.ts`: the panel's Date row
  // mounts a `Tooltip.Root` (`inspector-facts-relabel` design D3), which
  // reads its provider from context and throws without one. In the running
  // app that provider is the frame's (`ui/sidebar/sidebar-provider.svelte`),
  // an ancestor no unit test mounts — the same reason
  // `tags/TooltipHarness.svelte` gives.
  //
  // This does not wrap Inspector itself: several tests here call
  // `instance.startEditTags()` the instant `mount()` returns, before any
  // effect (including a `bind:this`) has run, so Inspector has to stay the
  // mounted root. Instead this captures the provider's context (`Map`,
  // `getAllContexts`'s documented use for handing existing context to a
  // programmatically-mounted component) once, for the test file to pass as
  // `mount`'s own `context` option when it mounts Inspector directly.
  import * as Tooltip from '$lib/components/ui/tooltip'
  import CaptureTooltipContext from './CaptureTooltipContext.test-harness.svelte'

  interface Props {
    oncaptured: (context: Map<unknown, unknown>) => void
  }

  let { oncaptured }: Props = $props()
</script>

<Tooltip.Provider delayDuration={0}>
  <CaptureTooltipContext {oncaptured} />
</Tooltip.Provider>
