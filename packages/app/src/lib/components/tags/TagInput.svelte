<script lang="ts">
  // One component, two placements (design D12): the toolbar's tag search and the
  // inspector's tag editor. Every rule it obeys is in `$lib/domain/tag-input`,
  // tested there; what is left here is event wiring, which this repo has no
  // harness for.
  import { tagSuggestions, vocabulary } from '$lib/api'
  import * as Command from '$lib/components/ui/command'
  import { Input } from '$lib/components/ui/input'
  import * as Popover from '$lib/components/ui/popover'
  import { Textarea } from '$lib/components/ui/textarea'
  import {
    applySuggestion,
    completeToken,
    confirmAction,
    filterSuggestions,
    initialHighlight,
    moveHighlight,
    SUGGESTION_FETCH,
    suggestionPrefix,
    suggestsOnFocus,
    type TagInputText,
  } from '$lib/domain/tag-input'
  import {
    KEY_DOWN,
    KEY_ENTER,
    KEY_ESCAPE,
    KEY_TAB,
    KEY_UP,
  } from '$lib/keyboard'
  import { portalTarget } from '$lib/portal'
  import { CATEGORY_TEXT_CLASS } from './categories'

  /** The default source: the library's whole vocabulary (design D12). */
  async function libraryTags(prefix: string, limit: number): Promise<string[]> {
    return (await tagSuggestions(prefix, limit)).map((tag) => tag.name)
  }

  interface Props {
    value: string
    /** So the frame's `/` binding can find the toolbar's field by id. */
    id?: string
    label: string
    placeholder?: string
    class?: string
    /** The text changed — by a keystroke or by an accepted suggestion. */
    oninput?: () => void
    /** A confirmation with nothing left to finish (design D13). */
    onsubmit?: () => void
    /** `Escape` with the list already closed: the map's way out of a field. */
    onescape?: (event: KeyboardEvent) => void
    /**
     * Where the list comes from. The library's whole vocabulary by default; the
     * bulk remove field narrows it to the selection's own tags
     * (`selection-and-bulk` D8), which no query against the library can answer.
     */
    suggest?: (prefix: string, limit: number) => Promise<string[]>
    /**
     * A textarea that grows with its text instead of a one-line input. For the
     * image's tag editor (spec `tag-editing`): an image carries dozens of tags,
     * and a line that scrolls sideways hides all but a few of them. Enter is
     * the confirmation here as everywhere, never a line break.
     */
    multiline?: boolean
  }

  let {
    value = $bindable(),
    id,
    label,
    placeholder,
    class: className,
    oninput,
    onsubmit,
    onescape,
    suggest = libraryTags,
    multiline = false,
  }: Props = $props()

  let field = $state<HTMLInputElement | HTMLTextAreaElement | null>(null)

  /** Why this portals where it does: `$lib/portal`'s `portalTarget` doc comment. */
  const portalTo = $derived(portalTarget(field))

  /**
   * Lets an owner (the Inspector) take the focus off the field once a submit it
   * handles itself is done with it — this component has no opinion on when that
   * is safe.
   */
  export function blur() {
    field?.blur()
  }

  /**
   * Puts the caret at the end of the field's text (design D5): the inspector's
   * `startEditTags` calls this once the field has mounted, so opening the
   * editor puts the caret on the fresh token `editorText`'s trailing space
   * leaves, rather than at the start where a click would land it.
   *
   * `field.focus()` above fires the field's own `onfocus`, which reads the
   * caret before the line below moves it — often 0, on a value that ends in
   * `editorText`'s trailing space either way, so the token there is always
   * empty. `suggestsOnFocus` (below, `onfocus`'s own guard) is what keeps
   * that from opening the list unasked: left open, the first `Escape` would
   * close the popover instead of cancelling the edit (`onescape` only fires
   * once the list is already closed).
   */
  export function focusEnd() {
    field?.focus()
    field?.setSelectionRange(value.length, value.length)
  }
  let open = $state(false)
  let suggestions = $state<string[]>([])
  /** `-1` is "nothing highlighted", which is what makes a confirmation submit. */
  let highlight = $state(-1)
  /** Answers to a prefix the user has since typed past are dropped. */
  let asked = 0

  const caretNow = () => field?.selectionStart ?? value.length

  function close() {
    open = false
    highlight = -1
  }

  async function refresh() {
    const caret = caretNow()
    const prefix = suggestionPrefix(value, caret)
    if (prefix === null) {
      close()
      return
    }

    const request = ++asked
    let names: string[]
    try {
      names = await suggest(prefix, SUGGESTION_FETCH)
    } catch {
      // A library that cannot answer is not worth a message over the input; the
      // command that matters — the search or the save — reports for itself.
      close()
      return
    }
    if (request !== asked) return

    suggestions = filterSuggestions(value, caret, names)
    highlight = initialHighlight(prefix, suggestions.length)
    open = suggestions.length > 0
  }

  /** Writes a rule's result back, caret included, and tells the owner. */
  function write(text: TagInputText) {
    value = text.value
    // After the binding has painted, or the caret lands in the old text.
    queueMicrotask(() => field?.setSelectionRange(text.caret, text.caret))
    oninput?.()
  }

  function accept(tag: string) {
    write(applySuggestion(value, caretNow(), tag))
    // The list closes on accept, so the next confirmation finishes the token
    // rather than accepting whatever the refreshed list would have highlighted.
    close()
    field?.focus()
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === KEY_ESCAPE) {
      if (!open) {
        onescape?.(event)
        return
      }
      event.preventDefault()
      close()
      return
    }

    if (open && (event.key === KEY_DOWN || event.key === KEY_UP)) {
      event.preventDefault()
      highlight = moveHighlight(highlight, event.key === KEY_DOWN ? 1 : -1, suggestions.length)
      return
    }

    const highlighted = open && highlight >= 0

    if (event.key === KEY_TAB && highlighted) {
      event.preventDefault()
      accept(suggestions[highlight])
      return
    }

    if (event.key !== KEY_ENTER) return
    event.preventDefault()
    const caret = caretNow()
    switch (confirmAction(value, caret, highlighted)) {
      case 'accept':
        accept(suggestions[highlight])
        break
      case 'complete':
        write(completeToken(value, caret))
        close()
        break
      case 'submit':
        close()
        onsubmit?.()
        break
    }
  }

  const fieldProps = $derived({
    id,
    placeholder,
    'class': className,
    'aria-label': label,
    'role': 'combobox',
    'aria-expanded': open,
    'aria-autocomplete': 'list',
    'autocomplete': 'off',
    'autocorrect': 'off',
    'spellcheck': false,
    'oninput': () => {
      oninput?.()
      void refresh()
    },
    'onfocus': () => {
      if (suggestsOnFocus(value, caretNow())) void refresh()
    },
    'onblur': close,
    onkeydown,
  } as const)
</script>

<!--
  One attribute set for both elements, spread: the textarea is the same field
  with room to grow, and two lists drift.
-->
{#if multiline}
  <Textarea bind:ref={field} bind:value {...fieldProps} />
{:else}
  <Input bind:ref={field} bind:value {...fieldProps} />
{/if}

<Popover.Root bind:open>
  <!--
    The input keeps the focus while the list is open: it is the thing being
    typed into, and the arrows are handled above. `escapeKeydownBehavior` is
    ignored for the same reason — `Escape` closes the list here and leaves the
    field on the second press (design D13).
  -->
  <Popover.Content
    customAnchor={field}
    portalProps={{ to: portalTo }}
    align="start"
    sideOffset={4}
    trapFocus={false}
    escapeKeydownBehavior="ignore"
    onOpenAutoFocus={(event) => event.preventDefault()}
    onCloseAutoFocus={(event) => event.preventDefault()}
    onmousedown={(event) => event.preventDefault()}
    class="w-(--bits-floating-anchor-width) p-0"
  >
    <Command.Root shouldFilter={false} disablePointerSelection>
      <Command.List>
        {#each suggestions as tag, index (tag)}
          <!--
            `data-selected` is the command's own highlight, which always lands on
            the first row; this list's highlight is `tag-input`'s and can be on
            no row at all, so the upstream one is painted out — but only on rows
            that are not highlighted: a variant class outranks the plain one, so
            painting it out everywhere left the first row blank when it was the
            highlighted one.

            The text colour is the vocabulary's (`tag-vocabulary` design D6),
            same lookup as the sidebar and the badges, so a suggestion reads
            as its category before it is even accepted.
          -->
          <Command.Item
            value={tag}
            class="
              cursor-pointer
              {index === highlight ? 'bg-muted text-foreground' : 'data-selected:bg-transparent'}
              {CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]}
            "
            onSelect={() => accept(tag)}
          >
            {tag}
          </Command.Item>
        {/each}
      </Command.List>
    </Command.Root>
  </Popover.Content>
</Popover.Root>
