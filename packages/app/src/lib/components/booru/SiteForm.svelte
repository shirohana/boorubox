<script lang="ts">
  // Settings · Booru, the editing half (design D13). One form for adding and
  // editing: the fields and the refusals are the same either way, and Rust
  // decides both — the address must be unique and the id never changes, so
  // nothing here validates a second time.
  import type { BooruSite } from '@boorubox/shared'
  import { booruSites, booruSiteSave, errorText } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'

  interface Props {
    /** The site being edited, or `null` to add one. */
    site: BooruSite | null
    onsaved: () => void
    oncancel: () => void
  }

  let { site, onsaved, oncancel }: Props = $props()

  let name = $state('')
  let baseUrl = $state('')
  let username = $state('')
  /**
   * Write-only (`booru-sites`): the key is in the operating system's store and
   * nothing reads it back, so this field starts blank on every edit and an
   * empty one means "keep whatever is there" — which is the `null` the command
   * takes.
   */
  let apiKey = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)

  // The fields follow whichever site the caller hands over, so opening Edit on
  // a second row while the first is open loads that row.
  $effect(() => {
    name = site?.name ?? ''
    baseUrl = site?.baseUrl ?? ''
    username = site?.username ?? ''
    apiKey = ''
    error = null
  })

  const insecure = $derived(baseUrl.trim() !== '' && !/^https:\/\//i.test(baseUrl.trim()))

  async function save() {
    if (saving) return
    const values = { name: name.trim(), baseUrl: baseUrl.trim(), username: username.trim() }
    const key = apiKey.trim()
    /** The row being edited, or `null` on a create. */
    const editedId = site?.id ?? null

    // A blank key means "keep the saved one" (`booru_site_save` takes `null`),
    // and a site being created has none to keep: it would be saved unable to
    // post, and say so only on the first upload.
    if (editedId === null && key === '') {
      error = 'A new site needs its API key: it is what authenticates the account.'
      return
    }

    saving = true
    error = null
    // Which rows existed before the save, so a rejection can be attributed to
    // the row this form wrote and to no other. Matching on the submitted values
    // instead would hand a duplicate-address refusal to the site that already
    // holds that address — marking a working site unusable.
    const before = new Set(booruSites.sites.map((candidate) => candidate.id))
    try {
      const saved = await booruSiteSave(
        editedId,
        values.name,
        values.baseUrl,
        values.username,
        key === '' ? null : key,
      )
      await booruSites.reload()
      // A key that was accepted is a credential store that answered, so
      // whatever it refused for this site before is over.
      if (key !== '') booruSites.usable(saved.id)
      onsaved()
    } catch (cause) {
      const reason = errorText(cause)
      error = reason
      // The row is written before the credential is touched (design D7), so a
      // rejection that left a row behind is the credential store's — the
      // refusals that happen first, a duplicate address among them, write no
      // such row. On a create that row is the one that was not there before; on
      // an edit it is this site, and only when the edit actually landed on it.
      await booruSites.reload()
      const written = editedId === null
        ? booruSites.sites.find((candidate) => !before.has(candidate.id))
        : booruSites.sites.find((candidate) => {
          return candidate.id === editedId
            && candidate.name === values.name
            && candidate.baseUrl === values.baseUrl
            && candidate.username === values.username
        })
      if (written) booruSites.refused(written.id, reason)
    } finally {
      saving = false
    }
  }
</script>

<form
  class="flex flex-col gap-3 rounded-lg border border-border p-3"
  onsubmit={(event) => {
    event.preventDefault()
    void save()
  }}
>
  <div class="grid gap-3 sm:grid-cols-2">
    <div class="grid gap-1.5">
      <Label for="site-name" class="text-xs text-muted-foreground">Name</Label>
      <Input id="site-name" class="h-8" bind:value={name} autocomplete="off" />
    </div>

    <div class="grid gap-1.5">
      <Label for="site-url" class="text-xs text-muted-foreground">
        Address
        <span class="font-normal">— the booru's root, without a path</span>
      </Label>
      <Input
        id="site-url"
        class="h-8 font-mono"
        placeholder="https://danbooru.donmai.us"
        bind:value={baseUrl}
        autocomplete="off"
        spellcheck={false}
      />
    </div>

    <div class="grid gap-1.5">
      <Label for="site-username" class="text-xs text-muted-foreground">Username</Label>
      <Input id="site-username" class="h-8" bind:value={username} autocomplete="off" />
    </div>

    <div class="grid gap-1.5">
      <Label for="site-api-key" class="text-xs text-muted-foreground">
        API key
        <span class="font-normal">
          {site ? '— blank keeps the saved one' : '— required'}
        </span>
      </Label>
      <Input
        id="site-api-key"
        type="password"
        class="h-8"
        bind:value={apiKey}
        autocomplete="off"
        spellcheck={false}
      />
    </div>
  </div>

  <p class="text-xs text-muted-foreground">
    The key goes to this computer's credential store, never to the library folder or the
    settings file — copying or syncing the library never carries it.
  </p>

  <!--
    `booru-sites`: warned, never refused. A self-hosted instance on a LAN is
    exactly what this feature exists for, and it often has no certificate.
  -->
  {#if insecure}
    <p class="text-xs text-amber-600 dark:text-amber-400">
      This address is not secure: the username and API key will be sent over the network in
      clear text.
    </p>
  {/if}

  {#if error}
    <p class="text-xs text-destructive">{error}</p>
  {/if}

  <div class="flex justify-end gap-2">
    <Button type="button" size="sm" variant="ghost" onclick={oncancel}>Cancel</Button>
    <Button type="submit" size="sm" disabled={saving}>
      {site ? 'Save site' : 'Add site'}
    </Button>
  </div>
</form>
