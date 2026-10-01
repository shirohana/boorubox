<script lang="ts">
  // Test support only: `Tooltip.Root` (bits-ui) reads its provider from context
  // and throws without one. In the running app that provider is the frame's
  // (`ui/sidebar/sidebar-provider.svelte`), an ancestor no unit test mounts, so
  // a component that draws a tooltip is mounted through this.
  import type { Component } from 'svelte'
  import * as Tooltip from '$lib/components/ui/tooltip'

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let { component: Inner, props }: { component: Component<any>, props: Record<string, unknown> } = $props()
</script>

<Tooltip.Provider delayDuration={0}>
  <Inner {...props} />
</Tooltip.Provider>
