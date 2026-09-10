<script lang="ts">
  // The upload form (design D9): a modal dialog rather than an inline panel,
  // because it is six fields and a preview and the Inspector's job is showing
  // the image's own facts. Every value it opens with is prefill from the image
  // (design D10) and editing one changes nothing in the library — the values
  // travel to the booru and back as a `posts` row, never into the image.
  import type {
    BooruSite,
    BooruUploadOutcome,
    ImageRecord,
    PostRef,
    UploadStep,
  } from '@boorubox/shared'
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link'
  import { booruUpload, errorText, openExternal, thumbnailUrl } from '$lib/api'
  import RatingControl from '$lib/components/tags/RatingControl.svelte'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Textarea } from '$lib/components/ui/textarea'
  import { postedName, postUrl, siteUrl } from './posted'
  import { checkUploadForm, prefillUploadForm, type UploadFormValues } from './upload-form'

  /** Each step in the words of what the booru was being asked to do (design D6). */
  const STEP_LABEL: Record<UploadStep, string> = {
    authenticate: 'The booru rejected the credential',
    createUpload: 'The booru refused the file',
    awaitProcessing: 'The booru did not finish processing the file in time',
    createPost: 'The post could not be created',
    commentary: 'The artist commentary could not be set',
  }

  interface Props {
    image: ImageRecord
    site: BooruSite
    open: boolean
    /** The post Rust recorded, for the caller to put into the loaded record. */
    onposted: (post: PostRef) => void
    onclose: () => void
  }

  let { image, site, open, onposted, onclose }: Props = $props()

  // Read once for the first paint; the effect below is what follows a later
  // image, and it is the only thing allowed to overwrite an edit in progress.
  // svelte-ignore state_referenced_locally
  let values = $state<UploadFormValues>(prefillUploadForm(image))
  let refusal = $state<string | null>(null)
  let running = $state(false)
  let outcome = $state<BooruUploadOutcome | null>(null)
  /**
   * A promise rejection, which is never a failure of the sequence: no library
   * open, the image or the site gone, or a credential the operating system
   * would not hand over — which is refused before any request is sent
   * (`booru-sites` design D7).
   *
   * Shown, never attributed: `AppError` crosses the wire as a bare string
   * (design D6), so nothing here can tell a credential refusal from "no library
   * open". Do not feed this to `booruSites.refused()` — it would mark a working
   * site unusable for the session on any of the other rejections. Design D7's
   * addendum narrows the store's refusals to test and save for that reason.
   */
  let rejection = $state<string | null>(null)
  /** Why the last outward link did not open; the browser is not this app's. */
  let linkFailure = $state<string | null>(null)
  let content = $state<HTMLElement | null>(null)

  const preview = $derived(image.missing ? null : thumbnailUrl(image.id))
  const failure = $derived(outcome?.outcome === 'failed' ? outcome.error : null)
  const posted = $derived(outcome?.outcome === 'posted' ? outcome : null)
  const commentaryFailure = $derived(
    posted?.commentary.status === 'failed' ? posted.commentary.message : null,
  )

  // The form follows whichever image and site the caller opens it for, so a
  // second upload from the same panel starts from that image's own values
  // rather than from the last one's edits.
  let shown = ''
  $effect(() => {
    const key = `${image.id}:${site.id}`
    if (key === shown) return
    shown = key
    values = prefillUploadForm(image)
    refusal = null
    outcome = null
    rejection = null
    linkFailure = null
  })

  async function send() {
    if (running) return
    const checked = checkUploadForm(values)
    if (!checked.ok) {
      refusal = checked.refusal
      return
    }

    refusal = null
    rejection = null
    outcome = null
    running = true
    try {
      // Design D6: every failure of the sequence itself comes back as a
      // resolved outcome naming its step. Only what `AppError` is for rejects.
      const result = await booruUpload(image.id, site.id, checked.form)
      outcome = result
      if (result.outcome === 'posted') onposted(result.post)
    } catch (cause) {
      rejection = errorText(cause)
    } finally {
      running = false
    }
  }

  async function openLink(url: string) {
    linkFailure = await openExternal(url)
  }
</script>

<Dialog.Root
  {open}
  onOpenChange={(next) => {
    // Not while it runs: the sequence carries on in Rust either way, and its
    // outcome — the post's id, or which step refused — is only reported here.
    if (!next && !running) onclose()
  }}
>
  <Dialog.Content
    bind:ref={content}
    class="max-h-[90vh] overflow-y-auto sm:max-w-2xl"
    onOpenAutoFocus={(event) => {
      // Not the first field: the dialog is a confirmation of an image tagged
      // already, so it opens with nothing armed. The focus lands on the
      // dialog itself, where Esc closes and Tab starts from the top.
      event.preventDefault()
      content?.focus()
    }}
  >
    <Dialog.Header>
      <Dialog.Title>Upload to {site.name}</Dialog.Title>
      <Dialog.Description>
        The file this library holds is what is posted; the booru is never asked to fetch it
        (design D3). Nothing here changes the image.
      </Dialog.Description>
    </Dialog.Header>

    <div class="flex gap-4">
      <div class="size-28 shrink-0 overflow-hidden rounded-lg border border-border bg-muted/40">
        {#await preview then src}
          {#if src}
            <img src={src} alt="" class="size-full object-contain" />
          {/if}
        {:catch}
          <span class="sr-only">No preview</span>
        {/await}
      </div>

      <dl class="grid min-w-0 grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs">
        <dt class="text-muted-foreground">Account</dt>
        <dd class="wrap-break-word">{site.username}</dd>
        <dt class="text-muted-foreground">Booru</dt>
        <dd class="wrap-break-word">{site.baseUrl}</dd>
        <dt class="text-muted-foreground">File</dt>
        <dd>{image.width} × {image.height} · {image.mime}</dd>
      </dl>
    </div>

    <div class="grid gap-1.5">
      <Label for="upload-tags" class="text-xs text-muted-foreground">Tags</Label>
      <TagInput
        id="upload-tags"
        bind:value={values.tags}
        label="Tags to post with"
        placeholder="Tags, separated by spaces"
        class="h-8"
      />
    </div>

    <div class="grid gap-1.5">
      <span class="text-xs text-muted-foreground">Rating</span>
      <!--
        Design D10: an unrated image opens with nothing chosen, and the send
        refuses until one is picked. The control writes nowhere — this is the
        one placement where choosing a rating is a form value, not an edit.
      -->
      <div>
        <RatingControl
          value={values.rating}
          onchoose={(rating) => {
            values.rating = rating
            return Promise.resolve()
          }}
        />
      </div>
    </div>

    <div class="grid gap-3 sm:grid-cols-2">
      <div class="grid gap-1.5">
        <Label for="upload-source" class="text-xs text-muted-foreground">Source</Label>
        <Input id="upload-source" class="h-8" bind:value={values.source} autocomplete="off" />
      </div>

      <div class="grid gap-1.5">
        <Label for="upload-artist" class="text-xs text-muted-foreground">
          Artist
          <span class="font-normal">— posted as a tag</span>
        </Label>
        <Input id="upload-artist" class="h-8" bind:value={values.artist} autocomplete="off" />
      </div>
    </div>

    <div class="grid gap-1.5">
      <Label for="upload-commentary-title" class="text-xs text-muted-foreground">
        Commentary title
      </Label>
      <Input
        id="upload-commentary-title"
        class="h-8"
        bind:value={values.commentaryTitle}
        autocomplete="off"
      />
    </div>

    <div class="grid gap-1.5">
      <Label for="upload-commentary-body" class="text-xs text-muted-foreground">
        Commentary
        <span class="font-normal">— left out when both fields are empty</span>
      </Label>
      <Textarea id="upload-commentary-body" rows={3} bind:value={values.commentaryBody} />
    </div>

    {#if refusal}
      <p class="text-xs text-destructive">{refusal}</p>
    {/if}

    {#if running}
      <!--
        FIXME(booru-upload D6): `booru_upload` runs the four steps behind one
        call and emits nothing while it does, so this names what the sequence
        does and never which step it is on — a moving label here would be a
        guess at the one fact the wait is about. An `upload:step` event from
        Rust is what naming it needs (webview handoff notes in tasks.md).
      -->
      <p class="text-xs text-muted-foreground">
        Uploading to {site.name}: handing over the file, waiting for it to be processed, then
        creating the post. This can take a minute.
      </p>
    {/if}

    {#if rejection}
      <p class="text-xs text-destructive">{rejection}</p>
    {/if}

    {#if failure}
      <!--
        Design D6: the step, then the booru's own words. A processing timeout
        also carries the upload it left behind, so the user can finish it there.
      -->
      <div class="rounded-lg border border-destructive/40 bg-destructive/5 p-3 text-xs">
        <p class="font-medium text-destructive">{STEP_LABEL[failure.step]}</p>
        <p class="mt-1 wrap-break-word">{failure.message}</p>
        <p class="mt-1 text-muted-foreground">Nothing was recorded against this image.</p>
        {#if failure.remoteRef && failure.step === 'awaitProcessing'}
          <button
            type="button"
            class="mt-1 inline-flex items-center gap-1 underline-offset-2 hover:underline"
            onclick={() => void openLink(siteUrl(site.baseUrl, `uploads/${failure.remoteRef}`))}
          >
            <span>Upload #{failure.remoteRef} on {site.name}</span>
            <ExternalLinkIcon class="size-3 shrink-0" />
          </button>
        {/if}
        {#if failure.remoteRef && failure.step === 'createUpload'}
          <!-- The file was refused because a post already holds it (design D12). -->
          {@const existingPost = failure.remoteRef}
          <button
            type="button"
            class="mt-1 inline-flex items-center gap-1 underline-offset-2 hover:underline"
            onclick={() => void openLink(postUrl(site.baseUrl, existingPost))}
          >
            <span>{postedName(site.name, existingPost)}</span>
            <ExternalLinkIcon class="size-3 shrink-0" />
          </button>
        {/if}
      </div>
    {/if}

    {#if posted}
      <div class="rounded-lg border border-border p-3 text-xs">
        <button
          type="button"
          class="inline-flex items-center gap-1 font-medium underline-offset-2 hover:underline"
          onclick={() => void openLink(postUrl(site.baseUrl, posted.post.remoteId))}
        >
          <span>{postedName(site.name, posted.post.remoteId)}</span>
          <ExternalLinkIcon class="size-3 shrink-0" />
        </button>

        <!--
          Design D6: the commentary is the one step whose failure is not the
          upload's. The post exists and is recorded; this is a warning naming
          it, and the commentary can be written on the booru.
        -->
        {#if commentaryFailure}
          <p class="mt-1 text-destructive">
            The post was created, but its artist commentary was not: {commentaryFailure}
          </p>
        {/if}
      </div>
    {/if}

    {#if linkFailure}
      <p class="text-xs text-destructive">This link could not be opened: {linkFailure}</p>
    {/if}

    <Dialog.Footer>
      <Button variant="ghost" disabled={running} onclick={onclose}>
        {posted ? 'Close' : 'Cancel'}
      </Button>
      {#if !posted}
        <Button disabled={running} onclick={send}>
          {running ? 'Uploading…' : 'Upload'}
        </Button>
      {/if}
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
