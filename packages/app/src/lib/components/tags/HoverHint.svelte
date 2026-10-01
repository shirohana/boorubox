<script lang="ts">
  // A mark that explains itself on hover: the trigger's content is the caller's,
  // the hint is `text`. The trigger is a `<span>`, not bits-ui's default
  // `<button>`: the host row, badge or chip is a button of its own and a button
  // cannot nest in one. `tabindex={-1}` keeps the mark out of the Tab order
  // (bits-ui defaults it to 0), since the hint is hover only. `shrink-0` keeps
  // the mark's width in a host's flex row. No provider is mounted here: the
  // frame's already wraps every mount point. `delayDuration={150}` overrides
  // that provider's 0, which suits the sidebar's icon buttons and not a text
  // that pops over a dense list: a pointer crossing the list does not pop every
  // hint it passes, and a pause on one reads at once.
  import type { Snippet } from 'svelte'
  import * as Tooltip from '$lib/components/ui/tooltip'

  interface Props {
    text: string
    /** Where the tooltip portals: set inside the viewer's `<dialog>`, left out elsewhere. */
    portalTo?: Element
    /** Added to the tooltip's own classes. */
    contentClass?: string
    children: Snippet
  }

  let { text, portalTo, contentClass, children }: Props = $props()
</script>

<Tooltip.Root delayDuration={150}>
  <Tooltip.Trigger tabindex={-1}>
    {#snippet child({ props })}
      <span {...props} class="inline-flex shrink-0 leading-none">{@render children()}</span>
    {/snippet}
  </Tooltip.Trigger>
  <Tooltip.Content portalProps={{ to: portalTo }} class={contentClass}>{text}</Tooltip.Content>
</Tooltip.Root>
