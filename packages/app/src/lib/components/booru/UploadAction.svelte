<script lang="ts">
  // Slot Inspector · actions, the upload half (`booru-upload`, design D9): the
  // action that opens the form, one entry per site this image is not already
  // on. The record for a site it *is* on is the `PostedLabel` above; showing
  // both would offer to do again what has been done (design D12).
  import type { ImageRecord, PostRef } from '@boorubox/shared'
  import { resolve } from '$app/paths'
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down'
  import CloudUploadIcon from '@lucide/svelte/icons/cloud-upload'
  import { booruSites } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { unpostedSites } from './posted'
  import UploadDialog from './UploadDialog.svelte'

  interface Props {
    image: ImageRecord
    /** The post Rust recorded, for the caller to put into the loaded record. */
    onposted: (post: PostRef) => void
  }

  let { image, onposted }: Props = $props()

  /** The site the form is open for; `null` when it is closed. */
  let uploading = $state<string | null>(null)

  // The list is read by the screen this action sits in, not here: the grid's
  // tiles need the same site names for their posted marks, and one reader per
  // screen is what keeps the two from disagreeing a moment after a site is
  // added.
  const targets = $derived(unpostedSites(image.posts, booruSites.sites))
  const site = $derived(uploading === null ? undefined : booruSites.site(uploading))
</script>

{#if booruSites.error}
  <!--
    The list is what says whether a site exists, so a list that could not be
    read cannot say "none configured" — that sentence would send the user to
    Settings to add the site they already have.
  -->
  <p class="text-xs text-destructive">
    The configured boorus could not be read: {booruSites.error}
  </p>
{:else if booruSites.sites.length === 0}
  <!--
    Spec `booru-upload`: no action at all when nothing is configured, and the
    place to configure one is named. A link, not a disabled button — app-frame's
    "no control appears before it does something".
  -->
  <p class="text-xs text-muted-foreground">
    No booru configured.
    <a href={resolve('/settings')} class="underline underline-offset-2">
      Add one in Settings · Booru
    </a>.
  </p>
{:else if targets.length === 1}
  <Button size="xs" variant="outline" onclick={() => (uploading = targets[0].id)}>
    <CloudUploadIcon />
    Upload to {targets[0].name}
  </Button>
{:else if targets.length > 1}
  <DropdownMenu.Root>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button size="xs" variant="outline" {...props}>
          <CloudUploadIcon />
          Upload to…
          <ChevronDownIcon />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content align="start">
      {#each targets as target (target.id)}
        <DropdownMenu.Item onSelect={() => (uploading = target.id)}>
          {target.name}
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Content>
  </DropdownMenu.Root>
{/if}

{#if site}
  <UploadDialog
    {image}
    {site}
    open={true}
    {onposted}
    onclose={() => (uploading = null)}
  />
{/if}
