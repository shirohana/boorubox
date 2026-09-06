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

  const current = $derived(image.rating ?? NONE)

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
  type="single"
  variant="outline"
  size="sm"
  value={current}
  disabled={saving}
  onValueChange={choose}
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
