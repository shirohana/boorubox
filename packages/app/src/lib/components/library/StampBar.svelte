<script lang="ts">
  // Slot Library · edit mode (`stamps` design D5): mounted by `LibraryScreen`
  // between its toolbar and the grid while edit mode is on. Reads `stamps`
  // directly rather than taking a list prop — `Sidebar.svelte` already keeps
  // it current on every library switch, so the bar only ever needs to read
  // `stamps.list`, the same shortcut `StampsSection` takes.
  import type { Stamp, TagEditSpec } from '@boorubox/shared'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import XIcon from '@lucide/svelte/icons/x'
  import { errorText, stamps, stampsDelete } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import * as Dialog from '$lib/components/ui/dialog'
  import { parseStamp } from '$lib/domain/stamp'
  import { blurOnEscape } from '$lib/keyboard'
  import StampForm from '$lib/components/stamps/StampForm.svelte'

  interface Props {
    /**
     * The one signal for what is active (owner's review, 2026-09-23): typed
     * directly, or filled by a chip's click or a saved dialog's `onsaved` —
     * there is no separate "activate" step, so cancelling is clearing this.
     * `LibraryScreen` derives the parsed stamp from it and clears it on
     * leaving the mode.
     */
    text: string
    /** For "Apply to N selected" — the screen's own `selection.count`. */
    selectionCount: number
    /** Raises the screen's confirm/write split over the active stamp's spec. */
    onapplyselection: () => void
    /** A stamp click Rust refused, from the screen (design D4). */
    error: string | null
  }

  let { text = $bindable(''), selectionCount, onapplyselection, error }: Props = $props()

  let field = $state<{ focusEnd: () => void } | null>(null)

  function clear() {
    text = ''
    field?.focusEnd()
  }

  const parsed = $derived(parseStamp(text))
  /**
   * A live hint while typing, not a refusal (owner's review, 2026-09-23):
   * blank reads as blank rather than as an error nobody asked for yet, and
   * this is the only place `parseStamp`'s reason for a bad field is shown —
   * `LibraryScreen`'s own derivation only needs pass/fail, not the reason.
   */
  const liveError = $derived(text.trim() !== '' && 'error' in parsed ? parsed.error : null)
  // Not `text.trim() !== '' && !('error' in parsed)`: `parseStamp` already
  // errors on an empty text (D1), so the trim check restated a rule the parse
  // already covers. `liveError` above keeps its own trim check — that one is
  // about suppressing a hint on a field nobody has typed into yet, a
  // different question from whether the field can apply.
  const canApply = $derived(!('error' in parsed))

  /** The stamp a chip's Edit… is open on, or `null` while none is. */
  let editing = $state<Stamp | null>(null)
  let savingNew = $state(false)
  /** The stamp a chip's Delete… asked about, or `null` while none has. */
  let deleting = $state<Stamp | null>(null)
  /** A create/edit/delete failure from the chip menus, apart from `error` above. */
  let manageError = $state<string | null>(null)

  /**
   * A saved stamp's text is checked before it can be saved (`StampForm`), so
   * this only returns `null` for a row a schema or grammar change left behind
   * — defensive, not a path any save can reach today.
   */
  function stampEdit(stamp: Stamp): TagEditSpec | null {
    const parsedStamp = parseStamp(stamp.text)
    return 'error' in parsedStamp ? null : parsedStamp.edit
  }

  function deleteConfirmed() {
    const doomed = deleting
    deleting = null
    if (doomed) void run(() => stampsDelete(doomed.id))
  }

  /** `StampsTable`'s own shape: re-read the list whether the write landed or not. */
  async function run(action: () => Promise<unknown>) {
    try {
      await action()
      manageError = null
    } catch (cause) {
      manageError = errorText(cause)
    }
    await stamps.refresh()
  }
</script>

<div class="flex flex-col gap-2 border-b border-border px-4 py-2">
  <!--
    The field first and full width (owner's review, 2026-09-23): it is the one
    place the active stamp is read from, so it leads the bar rather than
    trailing the saved stamps the way a one-off field used to.
  -->
  <div class="flex items-center gap-2">
    <!--
      The clear control is this bar's own markup, not a `TagInput` prop: the
      bar is the only field that wants one (the tag editor and the bulk dialog
      are multiline and submit on Enter), and a shared input should not grow a
      prop for one caller.
    -->
    <div class="relative min-w-0 flex-1">
      <TagInput
        bind:this={field}
        bind:value={text}
        label="Stamp"
        placeholder="cat animal -dog"
        class="h-7 w-full pr-7"
        onescape={blurOnEscape}
      />
      {#if text !== ''}
        <!--
          The centring is on this wrapper, not the button: the kit's button
          nudges itself with `translate` while pressed, which would replace a
          centring translate on the button and drop it out from under the
          pointer, so the release would miss and nothing would clear. The
          wrapper passes clicks beside the button through to the input.
        -->
        <div class="pointer-events-none absolute inset-y-0 right-0.5 flex items-center">
          <Button
            size="icon-xs"
            variant="ghost"
            aria-label="Clear stamp"
            class="pointer-events-auto"
            onclick={clear}
          >
            <XIcon />
          </Button>
        </div>
      {/if}
    </div>

    <Button size="sm" variant="ghost" onclick={() => (savingNew = true)}>Save as stamp…</Button>

    {#if selectionCount > 0}
      <Button size="sm" variant="outline" disabled={!canApply} onclick={onapplyselection}>
        Apply to {selectionCount.toLocaleString()} selected
      </Button>
    {/if}
  </div>

  {#if liveError}
    <p class="text-xs text-muted-foreground">{liveError}</p>
  {/if}

  {#if stamps.list.length > 0}
    <div class="flex flex-wrap items-center gap-2">
      {#each stamps.list as stamp (stamp.id)}
        <ContextMenu.Root>
          <ContextMenu.Trigger>
            {#snippet child({ props })}
              {@const inactiveText = stampEdit(stamp) === null}
              <!--
                A `Button` whose look follows `text`, not `TogglePrimitive`'s own
                internal flip (`LibraryScreen`'s inspector button, same
                reasoning): the chip writes the field in both directions — fills
                it, or empties it when it already holds this text — and the
                field stays the one signal for what is active. A pressed
                control that cannot be pressed off would break the expectation
                its own look sets. Pressed by text equality, not by id — two
                stamps that happen to share text both read pressed together,
                honestly.

                `aria-disabled`, not `disabled`: this trigger owns the only
                Edit…/Delete… for a stamp whose text no longer parses, and a
                `disabled` button also closes its context menu and drops its
                `title`. Clicking it still fills the field — the live hint
                above then says why it will not apply.
              -->
              <Button
                size="sm"
                variant={stamp.text === text ? 'default' : 'outline'}
                aria-pressed={stamp.text === text}
                aria-disabled={inactiveText}
                title={inactiveText ? `“${stamp.text}” is not an edit any more.` : stamp.text}
                onclick={() => (text = stamp.text === text ? '' : stamp.text)}
                {...props}
                class="{props.class ?? ''} {inactiveText ? 'text-muted-foreground opacity-70' : ''}"
              >
                {stamp.name}
              </Button>
            {/snippet}
          </ContextMenu.Trigger>
          <ContextMenu.Content>
            <ContextMenu.Item onSelect={() => (editing = stamp)}>
              <PencilIcon />
              Edit…
            </ContextMenu.Item>
            <ContextMenu.Item variant="destructive" onSelect={() => (deleting = stamp)}>
              <Trash2Icon />
              Delete…
            </ContextMenu.Item>
          </ContextMenu.Content>
        </ContextMenu.Root>
      {/each}
    </div>
  {/if}

  <p class="text-xs text-muted-foreground">
    Type a stamp, or click a saved one to fill the field; click an image to apply it. Click the
    active stamp again, or clear the field, to stop. There is no undo — the inverse stamp is the
    way back.
  </p>

  {#if error || manageError}
    <p class="text-xs text-destructive">{error ?? manageError}</p>
  {/if}
</div>

<!--
  Outside every chip's menu — a closed context menu's content is unmounted,
  and one dialog per chip would be machinery the grid's own `CollectionNameDialog`
  already avoids for the same reason.
-->
<Dialog.Root open={savingNew} onOpenChange={(next) => { if (!next) savingNew = false }}>
  <Dialog.Content class="sm:max-w-sm">
    <Dialog.Header>
      <Dialog.Title>Save as stamp</Dialog.Title>
      <Dialog.Description>
        Keeps this edit for later, listed here and on the settings screen.
      </Dialog.Description>
    </Dialog.Header>
    <StampForm
      stamp={null}
      initialText={text}
      onsaved={(saved) => {
        savingNew = false
        text = saved.text
        void stamps.refresh()
      }}
      oncancel={() => (savingNew = false)}
    />
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root open={editing !== null} onOpenChange={(next) => { if (!next) editing = null }}>
  <Dialog.Content class="sm:max-w-sm">
    <Dialog.Header>
      <Dialog.Title>Edit “{editing?.name}”</Dialog.Title>
    </Dialog.Header>
    {#if editing}
      <StampForm
        stamp={editing}
        onsaved={(saved) => {
          // Edit… is management, not activation: it only sets the field when
          // the stamp being edited was already the active one — otherwise it
          // would overwrite a typed one-off nobody asked to replace. Read
          // before `editing` is cleared, since it names the stamp this save
          // was for.
          const wasActive = editing !== null && editing.text === text
          editing = null
          if (wasActive) text = saved.text
          void stamps.refresh()
        }}
        oncancel={() => (editing = null)}
      />
    {/if}
  </Dialog.Content>
</Dialog.Root>

<!-- A stamp deleted mid-review is not recoverable, so this asks — `StampsTable`'s own dialog. -->
<ConfirmDialog
  title="Delete “{deleting?.name}”?"
  description="The stamp is gone and stops appearing here and on the settings screen. Nothing it has
    already applied to an image changes."
  confirmLabel="Delete stamp"
  open={deleting !== null}
  onclose={() => (deleting = null)}
  onconfirm={deleteConfirmed}
/>
