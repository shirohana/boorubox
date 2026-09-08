<script lang="ts">
  // One component, two placements (design D12): the toolbar's tag search and the
  // inspector's tag editor. Every rule it obeys is in `$lib/domain/tag-input`,
  // tested there; what is left here is event wiring, which this repo has no
  // harness for.
  import { tagSuggestions } from '$lib/api'
  import * as Command from '$lib/components/ui/command'
  import { Input } from '$lib/components/ui/input'
  import * as Popover from '$lib/components/ui/popover'
  import {
    applySuggestion,
    completeToken,
    confirmAction,
    filterSuggestions,
    initialHighlight,
    moveHighlight,
    SUGGESTION_FETCH,
    suggestionPrefix,
    type TagInputText,
  } from '$lib/domain/tag-input'
  import {
    KEY_DOWN,
    KEY_ENTER,
    KEY_ESCAPE,
    KEY_TAB,
    KEY_UP,
  } from '$lib/keyboard'

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
  }: Props = $props()

  let field = $state<HTMLInputElement | null>(null)

  /**
   * The list is portalled to `<body>` unless the field is inside a native
   * modal dialog. `showModal()` puts the viewer in the top layer, which sits
   * above every z-index, so a list under `<body>` opened beneath it — visible
   * through the backdrop and unreachable. Inside one, the list has to be a
   * child of the dialog itself.
   */
  const portalTo = $derived(field?.closest('dialog') ?? undefined)

  /**
   * Lets an owner (the Inspector) take the focus off the field once a submit it
   * handles itself is done with it — this component has no opinion on when that
   * is safe.
   */
  export function blur() {
    field?.blur()
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
    const prefix = suggestionPrefix(value, caretNow())
    if (prefix === null) {
      close()
      return
    }

    const request = ++asked
    let names: string[]
    try {
      names = (await tagSuggestions(prefix, SUGGESTION_FETCH)).map((tag) => tag.name)
    } catch {
      // A library that cannot answer is not worth a message over the input; the
      // command that matters — the search or the save — reports for itself.
      close()
      return
    }
    if (request !== asked) return

    suggestions = filterSuggestions(value, names)
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
</script>

<Input
  bind:ref={field}
  bind:value
  {id}
  {placeholder}
  class={className}
  aria-label={label}
  role="combobox"
  aria-expanded={open}
  aria-autocomplete="list"
  autocomplete="off"
  autocorrect="off"
  spellcheck={false}
  oninput={() => {
    oninput?.()
    void refresh()
  }}
  onfocus={() => void refresh()}
  onblur={close}
  {onkeydown}
/>

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
          -->
          <Command.Item
            value={tag}
            class="
              cursor-pointer
              {index === highlight ? 'bg-muted text-foreground' : 'data-selected:bg-transparent'}
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
