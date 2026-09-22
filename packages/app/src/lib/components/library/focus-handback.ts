/**
 * Whether the DOM focus needs to be handed back to the grid's current card
 * (`browse-fixes` design D4): `active` is `null`, is `document.body`, or sits
 * inside `within` — an element that is going away, such as a closing menu.
 * `ImageCard`'s `onmenuclose` and `LibraryScreen`'s window click handler both ask
 * this, so the one rule that decides "nothing meant to keep the focus" is
 * written once.
 */
export function isOrphanedFocus(active: Element | null, within?: Element | null): boolean {
  return !active || active === document.body || (within?.contains(active) ?? false)
}
