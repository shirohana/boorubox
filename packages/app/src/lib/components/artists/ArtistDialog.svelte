<script lang="ts">
  // Edits or creates an artist entry from wherever its tag — or an image's
  // author — is seen (`artist-workflow` design D1, D2, D4): the sidebar row,
  // the pinned chip and the inspector badge for edit mode, the inspector's
  // Artist row's "Create artist…" for create mode. Mounted beside the menu
  // that opens it, not inside it: bits-ui destroys a menu's content the
  // moment the item that opened this closes it (`Inspector.svelte`'s
  // `CollectionNameDialog` mount is the same pattern).
  //
  // Stays mounted and follows `open` (the `CollectionNameDialog` shape),
  // rather than `{#if request}`: a bits-ui `Dialog` torn down while still
  // open keeps running its own close effects against derived state that no
  // longer exists, which logs `derived_inert` on every Cancel and submit.
  import type { ArtistApplyPreview, ArtistEntry } from '@boorubox/shared'
  import {
    artistPreview,
    artistRevision,
    artistsApply,
    artistsApplyPreview,
    artistsList,
    artistsUpsert,
    errorText,
    latestOnly,
    renameArtist,
    vocabulary,
  } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Textarea } from '$lib/components/ui/textarea'
  import type { ArtistDialogRequest, ArtistDialogStep } from './artist-dialog'
  import {
    canConfirm,
    confirmLabel,
    refusalText,
    renamesDerived,
    retryLabel,
    sameName,
    savePlan,
    underscored,
  } from './artist-dialog'
  import { lines } from './lines'

  interface Props {
    open: boolean
    /** What is being edited or created — a snapshot the host takes when the dialog opens. */
    request: ArtistDialogRequest | null
    /**
     * Where this dialog portals (design D3 of `browse-fixes`): the caller's
     * own `portalTarget` result, so opening it from inside the viewer lands
     * it in its `<dialog>` rather than underneath it — without this the
     * dialog opens in `<body>`, under the viewer's own top layer, and
     * cannot be seen or clicked there (the `CollectionNameDialog`/
     * `UploadDialog` precedent). Omitted by hosts never inside the viewer
     * (the sidebar's own mount).
     */
    portalTo?: Element
    /**
     * Dismissed — by Cancel, the overlay or `Esc` — or every step of a save
     * landed. The host clears its request here, never in `onsaved`.
     */
    onclose: () => void
    /**
     * A save wrote something — every step, or the steps before one that was
     * refused: the host refreshes the vocabulary and the search. Called once
     * per attempt, after its last step ran, not once per step: a refresh
     * racing the next step would re-read a half-written save. The dialog
     * stays open on a refusal, so this must not close it.
     */
    onsaved: () => void
  }

  let { open, request, portalTo, onclose, onsaved }: Props = $props()

  let nameInput = $state<HTMLInputElement | null>(null)
  let name = $state('')
  let urlsText = $state('')
  let applyChecked = $state(false)
  let saving = $state(false)
  let error = $state<string | null>(null)

  /**
   * What the dialog reads once per open before a plan can be trusted — not
   * the debounced apply-preview below, which only ever adds information to a
   * line, never decides what a save does. `carriers` is the request tag's
   * carrier count: the edit's rename count, or in create mode the derived
   * tag's, which a create that renames it retags. `initialUrls` is the edit's
   * dirtiness baseline for {@link canConfirm}: the stored URLs only, never
   * the appended candidate, which is itself the edit being offered.
   * `urls` is what an edit's textarea opens with (the stored URLs, then the
   * candidate); a create's textarea is its request's URL from the start.
   * `entries` is what create mode's union reads.
   */
  interface Seed {
    carriers: number
    urls: string[]
    initialUrls: string[]
    entries: ArtistEntry[]
  }

  let seed = $state<Seed | null>(null)
  const carriers = $derived(seed?.carriers ?? null)
  const previewKnown = $derived(seed !== null)

  async function readSeed(request: ArtistDialogRequest): Promise<Seed> {
    if (request.mode === 'edit') {
      const preview = await artistPreview({ tag: request.tag, adapter: request.adapter })
      return {
        carriers: preview.carriers,
        urls: [...preview.urls, ...(preview.candidate ? [preview.candidate] : [])],
        initialUrls: preview.urls,
        entries: [],
      }
    }
    const [entries, preview] = await Promise.all([
      artistsList(),
      artistPreview({ tag: request.tag, adapter: null }),
    ])
    return { carriers: preview.carriers, urls: [], initialUrls: [], entries }
  }

  /**
   * A fast close and reopen must not let the first open's answer land on the
   * second: only the latest open's read is applied.
   */
  const seedTicket = latestOnly<Seed>()

  /**
   * The steps of a refused save not yet landed, set only once one of its
   * steps has landed; `null` otherwise. A retry runs these rather than
   * re-planning: a landed rename has already consumed the name it was planned
   * from, and planning again from the fields would rename a tag that no
   * longer exists. While it is set the fields are read-only — the retry does
   * not read them. A refused first step leaves it `null`, so the retry is
   * planned afresh from the fields the user corrected.
   */
  let pending = $state<ArtistDialogStep[] | null>(null)

  /** The steps landed since this open, which the refusal line names. */
  let landed = $state<ArtistDialogStep[]>([])

  /** Whether the next Save runs a step of this kind: any before a save, only those left after. */
  function leftToRun(kind: ArtistDialogStep['kind']): boolean {
    return pending === null || pending.some((step) => step.kind === kind)
  }

  // Follows whichever request the caller opens the dialog for (the
  // `CollectionNameDialog` pattern): `request` is the host's own snapshot,
  // not a live image, so it only changes on a fresh `open` transition, never
  // mid-save.
  $effect(() => {
    if (!open || !request) return
    name = request.tag
    urlsText = request.mode === 'create' ? request.url : ''
    applyChecked = false
    saving = false
    error = null
    seed = null
    pending = null
    landed = []
    applyPreview = null

    const opened = request
    void (async () => {
      try {
        const result = await seedTicket(readSeed(opened))
        if (!result.current) return
        seed = result.value
        if (opened.mode === 'edit') urlsText = result.value.urls.join('\n')
      } catch (cause) {
        error = errorText(cause)
      }
    })()
  })

  /**
   * The apply line's own counts (`artist-workflow` design D6), re-read 300 ms
   * after the URL lines settle rather than on every keystroke, and only the
   * still-current read is kept (`latestOnly`, `artist-workflow` design D5):
   * typing costs one scan per pause, not per key, and a stale answer from a
   * URL set the user has since edited away from is never shown.
   * `superseded` covers the gap `latestOnly` leaves: until the next read
   * starts, 300 ms later, the last one is still "latest", and its answer —
   * for the URLs of a closed or edited-away request — must not land on the
   * fresh state.
   */
  const applyPreviewTicket = latestOnly<ArtistApplyPreview>()
  let applyPreview = $state<ArtistApplyPreview | null>(null)

  $effect(() => {
    if (!open) return
    const urls = lines(urlsText)
    if (urls.length === 0) {
      applyPreview = null
      return
    }
    let superseded = false
    const timer = setTimeout(() => {
      void (async () => {
        try {
          const result = await applyPreviewTicket(artistsApplyPreview(urls))
          if (result.current && !superseded) applyPreview = result.value
        } catch {
        // Advisory only — the line simply stays at its last known answer;
          // the refusal that matters is the save's own, shown below.
        }
      })()
    }, 300)
    return () => {
      superseded = true
      clearTimeout(timer)
    }
  })

  /** The apply line's name for the URL lines: a create starts from one image's profile. */
  const source = $derived(request?.mode === 'create' ? 'this profile' : 'these URLs')

  /** Create mode: whether the save renames the derived tag, which the rename line says. */
  const renaming = $derived.by(() => {
    if (request?.mode !== 'create') return false
    const derived = request.tag
    return renamesDerived({ derived, name, derivedExists: vocabulary.categoryOf(derived) === 'artist' })
  })

  const planLabel = $derived(
    request?.mode === 'create'
      ? confirmLabel({ mode: 'create', applyImages: applyPreview?.images ?? null, renaming, carriers })
      : confirmLabel({ mode: 'edit', from: request?.tag ?? '', name, carriers }),
  )
  const label = $derived(pending !== null ? retryLabel(pending) : planLabel)

  const planConfirmable = $derived(
    request?.mode === 'create'
      ? canConfirm({ mode: 'create', saving, previewKnown })
      : canConfirm({
        mode: 'edit',
        saving,
        previewKnown,
        from: request?.tag ?? '',
        name,
        urls: lines(urlsText),
        initialUrls: seed?.initialUrls ?? [],
      }),
  )
  /** A retry runs what is left, never the fields, so only a save under way blocks it. */
  const confirmable = $derived(pending !== null ? !saving : planConfirmable)

  async function runStep(step: ArtistDialogStep): Promise<void> {
    if (step.kind === 'upsert') await artistsUpsert(step.entry)
    else if (step.kind === 'rename') await renameArtist(step.input)
    else await artistsApply(step.tag)
  }

  function plan(request: ArtistDialogRequest): ArtistDialogStep[] {
    const urls = lines(urlsText)
    return request.mode === 'edit'
      ? savePlan({ mode: 'edit', from: request.tag, name, urls, apply: applyChecked })
      : savePlan({
        mode: 'create',
        derived: request.tag,
        name,
        urls,
        entries: seed?.entries ?? [],
        derivedExists: vocabulary.categoryOf(request.tag) === 'artist',
      })
  }

  /**
   * Rust refuses an empty name, a name a non-artist category already owns, a
   * URL that does not normalise and a URL another artist owns, each naming
   * the offender, and that refusal is what is shown: a copy of those checks
   * here would be a second definition of what a valid entry is. A refusal
   * after a landed step (apply, after a saved entry) keeps what landed and
   * the dialog open on the reason, with only the unlanded steps left for
   * Save to retry (`artist-workflow` design D4).
   */
  async function save() {
    if (!request || saving) return
    saving = true
    error = null
    const steps = [...(pending ?? plan(request))]
    let landedNow = false
    let done = false
    try {
      while (steps.length > 0) {
        await runStep(steps[0])
        landed.push(steps.shift()!)
        landedNow = true
        artistRevision.bump()
      }
      pending = null
      done = true
    } catch (cause) {
      pending = landed.length > 0 ? steps : null
      error = refusalText({ landed, failed: steps[0], reason: errorText(cause) })
    } finally {
      saving = false
    }
    if (landedNow) onsaved()
    if (done) onclose()
  }
</script>

<Dialog.Root {open} onOpenChange={(next) => { if (!next) onclose() }}>
  <Dialog.Content
    portalProps={{ to: portalTo }}
    class="sm:max-w-sm"
    onOpenAutoFocus={(event) => {
      // Selected, not only focused: overtyping the wrong name is the point of
      // this dialog, and a caret left at one end would make that a select-all
      // first (`artist-workflow` design D4). Done here,
      // not in `onMount` (the `UploadDialog` precedent for `onOpenAutoFocus`):
      // bits-ui's own autofocus runs after mount and would otherwise land the
      // caret before this call does.
      event.preventDefault()
      nameInput?.select()
    }}
  >
    <Dialog.Header>
      <Dialog.Title>{request?.mode === 'create' ? 'Create artist' : 'Edit artist'}</Dialog.Title>
      <Dialog.Description>
        {#if request?.mode === 'create'}
          Records “{underscored(name)}” as the owner of this profile and tags every stored image
          from it. A future capture from this profile is tagged “{underscored(name)}” too.
        {:else}
          Records the name as the owner of these URLs; a new name also retags every image that
          carries “{request?.tag}”. A future capture from these URLs is tagged the saved name.
        {/if}
      </Dialog.Description>
    </Dialog.Header>

    <form
      class="flex flex-col gap-3"
      onsubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <div class="grid gap-1.5">
        <Label for="artist-dialog-name" class="text-xs text-muted-foreground">Name</Label>
        <Input
          id="artist-dialog-name"
          bind:ref={nameInput}
          bind:value={name}
          readonly={pending !== null}
          autocomplete="off"
        />
      </div>

      <div class="grid gap-1.5">
        <Label for="artist-dialog-urls" class="text-xs text-muted-foreground">
          Profile URLs
          <span class="font-normal">— one per line</span>
        </Label>
        <Textarea
          id="artist-dialog-urls"
          bind:value={urlsText}
          readonly={pending !== null}
          class="min-h-16 font-mono text-xs"
          spellcheck={false}
        />
      </div>

      {#if request && request.mode === 'edit' && (leftToRun('rename') || leftToRun('upsert'))}
        {#if carriers === null && !error}
          <!-- Only while there is no error: a preview refusal is the line to show instead. -->
          <p class="text-xs text-muted-foreground">
            Reading how many images carry “{request.tag}”…
          </p>
        {:else if carriers !== null}
          <p class="text-xs text-muted-foreground">
            {carriers.toLocaleString()} {carriers === 1 ? 'image carries' : 'images carry'}
            “{request.tag}”{sameName(request.tag, name) ? '' : ` and will be retagged “${underscored(name)}”`}.
          </p>
        {/if}
      {/if}

      {#if request && request.mode === 'create' && renaming && carriers !== null && leftToRun('rename')}
        <!--
          Said before the save rather than discovered after it: this create
          retags every carrier of the derived tag, trash included
          (`artist-workflow` design D4's create-mode rule).
        -->
        <p class="text-xs text-muted-foreground">
          “{request.tag}” is renamed to “{underscored(name)}” on {carriers.toLocaleString()}
          {carriers === 1 ? 'image' : 'images'}.
        </p>
      {/if}

      {#if lines(urlsText).length > 0 && leftToRun('apply')}
        <!--
          The apply line (`artist-workflow` design D6): create mode shows it
          alone (there is no checkbox — a create always applies, the count is
          only ever information here); edit mode pairs it with the opt-in
          checkbox, unchecked on every open (owner, 2026-09-28: no migration
          by default).
        -->
        <div class="flex items-start gap-2">
          {#if request?.mode === 'edit' && pending === null}
            <Checkbox
              id="artist-dialog-apply"
              checked={applyChecked}
              onCheckedChange={(checked) => (applyChecked = checked === true)}
            />
          {/if}
          <div class="grid gap-0.5">
            {#if request?.mode === 'edit' && pending === null}
              <Label for="artist-dialog-apply" class="text-xs font-normal">
                Apply to existing images
              </Label>
            {/if}
            <p class="text-xs text-muted-foreground">
              {#if applyPreview}
                {applyPreview.images.toLocaleString()}
                {applyPreview.images === 1 ? 'image comes' : 'images come'} from {source};
                {applyPreview.untagged.toLocaleString()} of
                {applyPreview.images === 1 ? 'it' : 'them'}
                {applyPreview.untagged === 1 ? 'carries' : 'carry'} no artist tag.
              {:else}
                Reading how many stored images come from {source}…
              {/if}
            </p>
          </div>
        </div>
      {/if}

      {#if error}
        <p class="text-xs text-destructive">{error}</p>
      {/if}

      <Dialog.Footer>
        <Button type="button" variant="ghost" onclick={onclose}>Cancel</Button>
        <Button type="submit" disabled={!confirmable}>{label}</Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
