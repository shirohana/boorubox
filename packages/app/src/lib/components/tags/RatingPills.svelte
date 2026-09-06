<script lang="ts">
  // Slot Sidebar · filters (design D17). The counts arrive with the rating
  // clause of the search dropped (design D8), so a pill answers "how many would
  // I get if I asked for this instead", not "how many are showing" — which is
  // why they are non-zero while a rating is already in the query.
  import type { RatingCounts } from '@boorubox/shared'
  import { ratingLabel } from '$lib/domain/format'
  import { parseTagSearch, toggleRatingInQuery } from '$lib/domain/tag-utils'
  import { RATING_COLOUR, RATING_COLOUR_DIM, RATINGS } from './ratings'

  interface Props {
    /** `null` while a search is running (design D8): the pills stay mounted with blank counts. */
    counts: RatingCounts | null
    tagQuery: string
    onquery: (next: string) => void
  }

  let { counts, tagQuery, onquery }: Props = $props()

  const parsed = $derived(parseTagSearch(tagQuery))
  const asked = $derived(new Set(parsed.ratings))
</script>

<section class="p-2">
  <h2 class="px-1 pb-1 text-xs font-medium text-muted-foreground">Rating</h2>

  <!--
    A grid, not a wrapping row: a count growing a digit changed a pill's width
    and pushed the row onto two lines, so the whole sidebar jumped on a click.
  -->
  <div class="grid grid-cols-4 gap-1 px-1">
    {#each RATINGS as rating (rating)}
      <button
        type="button"
        aria-pressed={asked.has(rating)}
        title={counts ? `${ratingLabel(rating)} — ${counts[rating]}` : ratingLabel(rating)}
        class="
          flex items-center justify-between gap-1 rounded-md border px-1.5 py-0.5 text-xs
          {asked.has(rating)
            ? `border-transparent ${RATING_COLOUR[rating]}`
            : `${RATING_COLOUR_DIM[rating]} hover:text-foreground`}
        "
        onclick={() => onquery(toggleRatingInQuery(tagQuery, rating))}
      >
        <span class="font-semibold uppercase">{rating}</span>
        <span class="tabular-nums">{counts ? counts[rating] : '–'}</span>
      </button>
    {/each}

    <button
      type="button"
      aria-pressed={parsed.includeUnrated}
      title={counts ? `unrated — ${counts.unrated}` : 'unrated'}
      class="
        col-span-4 flex items-center justify-between gap-1 rounded-md border px-1.5 py-0.5 text-xs
        {parsed.includeUnrated
          ? 'border-transparent bg-secondary text-secondary-foreground'
          : 'border-border text-muted-foreground hover:text-foreground'}
      "
      onclick={() => onquery(toggleRatingInQuery(tagQuery, 'unrated'))}
    >
      <span class="font-semibold">none</span>
      <span class="tabular-nums">{counts ? counts.unrated : '–'}</span>
    </button>
  </div>
</section>
