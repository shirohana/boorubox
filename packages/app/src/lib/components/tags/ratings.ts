// How a rating looks, in one place: the inspector's control and the tile's
// badge have to agree, and the badge is read as a colour rather than as a letter
// (spec `rating`, "distinguishable between the four ratings without reading").

import type { Rating } from '@boorubox/shared'

/** Least to most explicit, which is the order every control offers them in. */
export const RATINGS: Rating[] = ['g', 's', 'q', 'e']

/**
 * The booru convention: green is safe, red is not, and the two middles warm.
 * `!` on the fill: the shadcn toggle paints its own on-state and hover
 * background through variant classes, which outrank a plain one and left the
 * active rating as white text on grey.
 */
export const RATING_COLOUR: Record<Rating, string> = {
  g: 'bg-emerald-500! text-white',
  s: 'bg-amber-500! text-black',
  q: 'bg-orange-600! text-white',
  e: 'bg-rose-600! text-white',
}

/**
 * The same four colours, dimmed for an inactive control (tinted border and
 * text, no fill) so a rating stays identifiable by its colour even when it is
 * not the active one — used by `RatingPills` and `RatingControl`.
 */
export const RATING_COLOUR_DIM: Record<Rating, string> = {
  g: 'border-emerald-500/40 text-emerald-600 dark:text-emerald-400',
  s: 'border-amber-500/40 text-amber-600 dark:text-amber-400',
  q: 'border-orange-500/40 text-orange-600 dark:text-orange-400',
  e: 'border-rose-500/40 text-rose-600 dark:text-rose-400',
}
