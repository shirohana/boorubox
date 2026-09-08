<script lang="ts">
  // Slot Inspector · rating (design D17): the four ratings and the clear, with
  // the image's own shown. The write goes through the store, which swaps the
  // record it returns into the row it already occupies (design D10) — so the
  // tile's badge and this control never disagree.
  import type { Rating } from '@boorubox/shared'
  import type { SearchResults } from '$lib/api'
  import { errorText } from '$lib/api'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { ratingLabel } from '$lib/domain/format'
  import { KEY_LEFT, KEY_RIGHT } from '$lib/keyboard'
  import { RATING_COLOUR, RATING_COLOUR_DIM, RATINGS } from './ratings'

  interface Props {
    image: { id: string, rating: Rating | null }
    results: SearchResults
  }

  let { image, results }: Props = $props()

  /** Unrated is a value the user picks, not the absence of one (`is:unrated`). */
  const NONE = 'none'

  let saving = $state(false)
  let error = $state<string | null>(null)
  let group = $state<HTMLDivElement | null>(null)

  const current = $derived(image.rating ?? NONE)

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
    // A second click on the current choice deselects it, which the group reports
    // as an empty value; both it and `none` mean unrated.
    const rating = picked === NONE || picked === '' ? null : picked as Rating
    if (rating === image.rating) return
    saving = true
    error = null
    try {
      await results.saveRating(image.id, rating)
    } catch (cause) {
      error = errorText(cause)
    } finally {
      saving = false
    }
  }
</script>

<ToggleGroup.Root
  bind:ref={group}
  type="single"
  variant="outline"
  size="sm"
  value={current}
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
