// The keyboard map lives where it acts — grid keys in the grid, viewer keys in
// the lightbox (design D14). The one thing that must not be written twice is
// "is the user typing", because a guard that drifts between two regions turns
// typing an `i` into a toggled inspector.

/** Design D14: every binding in the map is spelled once, here. */
export const KEY_LEFT = 'ArrowLeft'
export const KEY_RIGHT = 'ArrowRight'
export const KEY_UP = 'ArrowUp'
export const KEY_DOWN = 'ArrowDown'
export const KEY_HOME = 'Home'
export const KEY_END = 'End'
export const KEY_ENTER = 'Enter'
export const KEY_SPACE = ' '
export const KEY_ESCAPE = 'Escape'
/** Toggles the inspector column in the grid, inspect mode in the lightbox. */
export const KEY_INSPECT = 'i'
/** Focuses the tag search field from anywhere in the frame. */
export const KEY_SEARCH = '/'
/**
 * With Cmd/Ctrl: whole-app zoom (`lib/zoom.ts`). `+` is what a shifted `=` sends
 * on some layouts.
 */
export const KEYS_ZOOM_IN = ['=', '+']
export const KEY_ZOOM_OUT = '-'
export const KEY_ZOOM_RESET = '0'
/** With Cmd/Ctrl: collapse or expand the sidebar. Bound by the sidebar provider, listed here. */
export const KEY_SIDEBAR = 'b'

export interface KeyBinding {
  where: string
  keys: string[]
  action: string
}

/**
 * The keyboard map as the settings screen shows it (spec `app-frame`, "One
 * keyboard map"). Display only — each binding still fires where it acts — but
 * kept beside the key names so the list and the bindings cannot drift apart
 * without the edit being visible here.
 */
export const KEYBOARD_MAP: KeyBinding[] = [
  { where: 'Anywhere', keys: ['/'], action: 'Focus the tag search' },
  { where: 'Anywhere', keys: ['⌘ B'], action: 'Collapse or expand the sidebar' },
  { where: 'Anywhere', keys: ['⌘ =', '⌘ -', '⌘ 0'], action: 'Zoom in, zoom out, reset zoom' },
  { where: 'Search field', keys: ['Esc'], action: 'Leave the field' },
  { where: 'Grid', keys: ['←', '→', '↑', '↓'], action: 'Move the focused card' },
  { where: 'Grid', keys: ['Home', 'End'], action: 'Focus the first or last card' },
  { where: 'Grid', keys: ['Enter', 'Space'], action: 'Open the focused image' },
  { where: 'Grid', keys: ['I'], action: 'Show or hide the inspector' },
  { where: 'Viewer', keys: ['←', '→'], action: 'Previous or next image' },
  { where: 'Viewer', keys: ['I'], action: 'Show or hide the inspector' },
  { where: 'Viewer', keys: ['Esc', 'Space'], action: 'Close, focusing the image shown last' },
]

const TYPING_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT'])

/**
 * True while the event is headed for a text field, where a shortcut would eat a
 * character instead of firing. Read the *target*, not the active element: a
 * bubbled event names the field it started in even after focus has moved on.
 *
 * The editable check walks to the nearest `contenteditable` ancestor rather
 * than reading `isContentEditable`, which jsdom does not implement — swapping
 * it back in would pass in the app and stop being covered by any test.
 */
export function isTypingTarget(event: KeyboardEvent | Event): boolean {
  const target = event.target
  if (!(target instanceof HTMLElement)) return false
  if (TYPING_TAGS.has(target.tagName)) return true

  const editable = target.closest('[contenteditable]')
  return editable !== null && editable.getAttribute('contenteditable') !== 'false'
}
