<script lang="ts">
  // Slot Inspector · rating (design D17), and the bulk rating in the selection
  // toolbar (`selection-and-bulk` D8). The write is the caller's: one image's
  // rating goes through the search store, a selection's through
  // `bulk_set_rating`, and the control is the same four choices and the same
  // clear in both.
  import type { Rating } from '@boorubox/shared'
  import { errorText } from '$lib/api'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { ratingLabel } from '$lib/domain/format'
  import { KEY_LEFT, KEY_RIGHT } from '$lib/keyboard'
  import { RATING_COLOUR, RATING_COLOUR_DIM, RATINGS } from './ratings'

  interface Props {
    /**
     * The choice shown as current. `null` is the explicit "unrated" one;
     * `undefined` is "there is no single current rating", which is a whole
     * selection and shows no choice as active.
     */
    value: Rating | null | undefined
    /** Writes the picked rating; what it means is the caller's business. */
    onchoose: (rating: Rating | null) => Promise<void>
    /**
     * A choice was made — before the write settles, because this is about the
     * keyboard and not about the result. The viewer's placement takes its focus
     * back here, since Space on a choice is the choice's press and would
     * otherwise stop closing the viewer (`browse-polish` design D4, amended).
     * Beside the grid nothing is passed and the focus stays on the choice.
     */
    onchosen?: () => void
  }

  let { value, onchoose, onchosen }: Props = $props()

  /** Unrated is a value the user picks, not the absence of one (`is:unrated`). */
  const NONE = 'none'

  let saving = $state(false)
  let error = $state<string | null>(null)
  let group = $state<HTMLDivElement | null>(null)

  const current = $derived(value === undefined ? '' : value ?? NONE)

  /**
   * What the group holds, written back from `current` after every choice. The
   * group keeps its own value once clicked, and for a whole selection nothing
   * ever comes back to correct it: the second click on a rating would deselect,
   * arrive here as `''`, and unrate every selected image. Mirroring the props
   * keeps every bulk click an explicit set.
   */
  let shown = $derived(current)

  /**
   * bits-ui offers roving focus and Tab-stepping as an either-or: its roving
   * branch owns the arrows and leaves one tab stop, `rovingFocus={false}` gives
   * every choice a tab stop and hands the arrows back. The spec asks for both,
   * so the arrow step is owned here (design D3).
   *
   * `preventDefault` is also the signal the viewer reads to keep its own arrows
   * off this control — including at the two ends, where the focus does not move
   * but the image still must not.
   */
  function stepFocus(event: KeyboardEvent) {
    const direction = event.key === KEY_LEFT ? -1 : event.key === KEY_RIGHT ? 1 : 0
    if (direction === 0 || !group) return

    const choices = [...group.querySelectorAll<HTMLElement>('[role="radio"]')]
    const target = event.target
    const from = target instanceof Node
      ? choices.findIndex((choice) => choice.contains(target))
      : -1
    if (from === -1) return

    event.preventDefault()
    choices[from + direction]?.focus()
  }

  async function choose(picked: string) {
    // Every path below is a choice the user made, including the one that writes
    // nothing, so the hook fires here rather than after the write.
    onchosen?.()
    // A second click on the current choice deselects it, which the group reports
    // as an empty value; both it and `none` mean unrated.
    const rating = picked === NONE || picked === '' ? null : picked as Rating
    if (rating === value) {
      shown = current
      return
    }
    saving = true
    error = null
    try {
      await onchoose(rating)
    } catch (cause) {
      error = errorText(cause)
    } finally {
      saving = false
      shown = current
    }
  }
</script>

<ToggleGroup.Root
  bind:ref={group}
  type="single"
  variant="outline"
  size="sm"
  bind:value={shown}
  disabled={saving}
  rovingFocus={false}
  onValueChange={choose}
  onkeydown={stepFocus}
  aria-label="Rating"
>
  {#each RATINGS as rating (rating)}
    <ToggleGroup.Item
      value={rating}
      aria-label={ratingLabel(rating)}
      title={ratingLabel(rating)}
      class={current === rating ? RATING_COLOUR[rating] : RATING_COLOUR_DIM[rating]}
    >
      {rating}
    </ToggleGroup.Item>
  {/each}
  <ToggleGroup.Item value={NONE} aria-label="unrated" title="unrated">none</ToggleGroup.Item>
</ToggleGroup.Root>

{#if error}
  <p class="mt-2 text-xs text-destructive">{error}</p>
{/if}
