<script lang="ts">
  // The inspector's upload section, after Collections (`booru-upload` design
  // D9): the action that opens the form, one entry per site this image is not
  // already on. The record for a site it *is* on is the `PostedLabel` in the
  // facts block below; showing both would offer to do again what has been
  // done (design D12).
  //
  // Two mounts, one component (`sidebar-inspector-polish` design D2): the
  // inspector places the button after Collections and the "no booru
  // configured" hint at the panel's foot, but both need the same reading of
  // `booruSites` to agree on whether a booru exists. `part` picks which of
  // the two this mount renders, so that one decision — is a booru
  // configured — is made in one place rather than copied at each call site.
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
    /**
     * Which of the two mounts this is (design D2): the button(s), or the
     * "no booru configured" hint.
     */
    part: 'action' | 'hint'
    /**
     * Where the site menu and the upload dialog portal (design D3 of
     * `browse-fixes`): the caller's own `portalTarget` result, so opening this
     * from inside the viewer lands them in its `<dialog>` rather than
     * underneath it.
     */
    portalTo?: Element
    /**
     * The post Rust recorded, for the caller to put into the loaded record.
     * Only the `'action'` mount can post, so only it passes one.
     */
    onposted?: (post: PostRef) => void
  }

  let { image, part, portalTo, onposted }: Props = $props()

  /**
   * The site the form is open for; `null` when it is closed. Owned by the
   * `'action'` mount, the only one that opens it.
   */
  let uploading = $state<string | null>(null)

  // The list is read by the screen this action sits in, not here: the grid's
  // tiles need the same site names for their posted marks, and one reader per
  // screen is what keeps the two from disagreeing a moment after a site is
  // added.
  const targets = $derived(unpostedSites(image.posts, booruSites.sites))
  const site = $derived(uploading === null ? undefined : booruSites.site(uploading))
</script>

{#if part === 'action'}
  {#if !booruSites.error && targets.length === 1}
    <section class="border-t border-border px-4 py-3">
      <Button size="xs" variant="outline" onclick={() => (uploading = targets[0].id)}>
        <CloudUploadIcon />
        Upload to {targets[0].name}
      </Button>
    </section>
  {:else if !booruSites.error && targets.length > 1}
    <section class="border-t border-border px-4 py-3">
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
        <DropdownMenu.Content align="start" portalProps={{ to: portalTo }}>
          {#each targets as target (target.id)}
            <DropdownMenu.Item onSelect={() => (uploading = target.id)}>
              {target.name}
            </DropdownMenu.Item>
          {/each}
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </section>
  {/if}

  {#if site}
    <UploadDialog
      {image}
      {site}
      open={true}
      {portalTo}
      onposted={(post) => onposted?.(post)}
      onclose={() => (uploading = null)}
    />
  {/if}
{:else if booruSites.error}
  <!--
    The list is what says whether a site exists, so a list that could not be
    read cannot say "none configured" — that sentence would send the user to
    Settings to add the site they already have.
  -->
  <section class="border-t border-border px-4 py-3">
    <p class="text-xs text-destructive">
      The configured boorus could not be read: {booruSites.error}
    </p>
  </section>
{:else if booruSites.sites.length === 0}
  <!--
    Spec `booru-upload`: no action at all when nothing is configured, and the
    place to configure one is named. A link, not a disabled button — app-frame's
    "no control appears before it does something".
  -->
  <section class="border-t border-border px-4 py-3">
    <p class="text-xs text-muted-foreground">
      No booru configured.
      <a href={resolve('/settings')} class="underline underline-offset-2">
        Add one in Settings · Booru
      </a>.
    </p>
  </section>
{/if}
