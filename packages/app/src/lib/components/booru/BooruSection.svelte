<script lang="ts">
  // Slot Settings · Booru (design D13): the boorus this library posts to, as a
  // sibling of Settings · Rules rather than a shared section — two unrelated
  // tables of configuration that happen to have been planned on one line.
  import type { BooruConnectionTest, BooruSite } from '@boorubox/shared'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { booruSiteDelete, booruSites, booruSiteTest, errorText, library } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Table from '$lib/components/ui/table'
  import SiteForm from './SiteForm.svelte'

  let formOpen = $state(false)
  let editing = $state<BooruSite | null>(null)
  let testing = $state<string | null>(null)
  let confirming = $state<BooruSite | null>(null)
  let error = $state<string | null>(null)

  // Sites belong to the library, so the list follows a switch — and the switch
  // can be started from the menu on this very screen. The path, not the whole
  // status: a capture replaces the status object and would re-read the list.
  const libraryPath = $derived(library.status?.libraryPath ?? null)
  $effect(() => {
    void booruSites.load(libraryPath)
  })

  /**
   * `booru_site_test` rejects, before reaching the network, exactly when the
   * credential itself could not be read (`booru-sites` design D7); every answer
   * it resolves with means the store handed the key over. That reject-or-
   * resolve split is the only credential signal the app has — see design D7's
   * addendum — so this is where a site becomes "unusable".
   */
  async function test(site: BooruSite) {
    testing = site.id
    error = null
    try {
      booruSites.tested(site.id, await booruSiteTest(site.id))
    } catch (cause) {
      booruSites.refused(site.id, errorText(cause))
    } finally {
      testing = null
    }
  }

  async function remove(site: BooruSite) {
    error = null
    try {
      await booruSiteDelete(site.id)
      booruSites.forget(site.id)
    } catch (cause) {
      error = errorText(cause)
    }
    await booruSites.reload()
  }

  function outcomeText(site: BooruSite, result: BooruConnectionTest): string {
    switch (result.status) {
      case 'connected':
        return 'Connected, and the account was accepted.'
      case 'credentialRejected':
        return 'The site answered and rejected the username or API key.'
      case 'unreachable':
        return `Could not reach ${site.baseUrl} — ${result.reason}`
    }
  }
</script>

<section class="flex flex-col gap-4">
  <h2 class="text-sm font-semibold">Booru</h2>

  <p class="text-sm text-muted-foreground">
    The boorus this library uploads to. The name, address and account are stored with the
    library; each site's API key is held by this computer's credential store, so a copied or
    synced library folder never carries it.
  </p>

  {#if booruSites.sites.length === 0}
    <p class="text-sm text-muted-foreground">
      No booru configured. Add one to offer an upload action in the inspector.
    </p>
  {:else}
    <div class="overflow-x-auto rounded-lg border border-border">
      <Table.Root>
        <Table.Header>
          <Table.Row>
            <Table.Head>Name</Table.Head>
            <Table.Head>Address</Table.Head>
            <Table.Head>Account</Table.Head>
            <Table.Head>Credential</Table.Head>
            <Table.Head class="w-32"></Table.Head>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each booruSites.sites as site (site.id)}
            {@const refusal = booruSites.refusal(site.id)}
            {@const result = booruSites.test(site.id)}
            <Table.Row>
              <Table.Cell class="align-top font-medium">{site.name}</Table.Cell>
              <Table.Cell class="align-top whitespace-normal">
                <code class="font-mono text-xs break-all">{site.baseUrl}</code>
              </Table.Cell>
              <Table.Cell class="align-top">{site.username}</Table.Cell>

              <!--
                `whitespace-normal` over the cell's default nowrap, and a wrap
                that may break inside a word: a test outcome carries the
                transport's whole sentence with a URL in it, and unwrapped it
                scrolls the table sideways and takes the row's buttons with it.
                The settings column is narrow enough that the address cell has
                to wrap as well.
              -->
              <Table.Cell class="align-top text-xs wrap-anywhere whitespace-normal">
                <!--
                  `booru-sites`: a site whose credential cannot be read is
                  unusable and says why. Nothing reads a credential until it is
                  asked to, so an untested site says that rather than claiming
                  it has one.
                -->
                {#if refusal}
                  <p class="text-destructive">Unusable — {refusal}</p>
                {:else if result}
                  <p class={result.status === 'connected' ? '' : 'text-destructive'}>
                    {outcomeText(site, result)}
                  </p>
                {:else}
                  <p class="text-muted-foreground">Not checked.</p>
                {/if}
              </Table.Cell>

              <Table.Cell class="align-top">
                <div class="flex justify-end gap-1">
                  <Button
                    size="xs"
                    variant="outline"
                    disabled={testing === site.id}
                    onclick={() => test(site)}
                  >
                    {testing === site.id ? 'Testing…' : 'Test'}
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label="Edit {site.name}"
                    onclick={() => {
                      editing = site
                      formOpen = true
                    }}
                  >
                    <PencilIcon />
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label="Remove {site.name}"
                    onclick={() => (confirming = site)}
                  >
                    <Trash2Icon />
                  </Button>
                </div>
              </Table.Cell>
            </Table.Row>
          {/each}
        </Table.Body>
      </Table.Root>
    </div>
  {/if}

  {#if formOpen}
    <SiteForm
      site={editing}
      onsaved={() => {
        formOpen = false
        editing = null
      }}
      oncancel={() => {
        formOpen = false
        editing = null
      }}
    />
  {:else}
    <div>
      <Button size="sm" onclick={() => (formOpen = true)}>
        <PlusIcon />
        New site
      </Button>
    </div>
  {/if}

  {#if booruSites.error}
    <p class="text-sm text-destructive">{booruSites.error}</p>
  {/if}

  {#if error}
    <p class="text-sm text-destructive">{error}</p>
  {/if}
</section>

<!--
  Removing a site takes its stored key with it (`booru-sites`), and that key may
  be the one another library configured for the same account (design D7). What
  it does not take is the record of what has been posted there.
-->
<ConfirmDialog
  title="Remove “{confirming?.name}”?"
  description="Its API key is removed from this computer's credential store. Images already
    posted to it keep saying so, without a link to the post."
  confirmLabel="Remove site"
  open={confirming !== null}
  onclose={() => (confirming = null)}
  onconfirm={() => {
    const doomed = confirming
    confirming = null
    if (doomed) void remove(doomed)
  }}
/>
