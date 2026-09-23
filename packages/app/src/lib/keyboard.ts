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
 * Accepts the highlighted tag suggestion (design D13). Not a row of the map
 * below and not a new binding: it acts only while the focus is in a text field,
 * which is where the map declares nothing fires.
 */
export const KEY_TAB = 'Tab'
/**
 * With Cmd/Ctrl: whole-app zoom (`lib/zoom.ts`). `+` is what a shifted `=` sends
 * on some layouts.
 */
export const KEYS_ZOOM_IN = ['=', '+']
export const KEY_ZOOM_OUT = '-'
export const KEY_ZOOM_RESET = '0'
/** Enters or leaves full screen (`lib/fullscreen.svelte.ts`). Types nothing, so unguarded. */
export const KEY_FULLSCREEN = 'F11'
/** With Cmd/Ctrl: collapse or expand the sidebar. Bound by the sidebar provider, listed here. */
export const KEY_SIDEBAR = 'b'
/**
 * With Cmd/Ctrl: select every image in the current result (spec `selection`).
 * Bound by the library screen, not by the grid: it is about the result, not
 * about the card the focus is on.
 */
export const KEY_SELECT_ALL = 'a'
/**
 * Move to the trash, unmodified: `Delete` is the key a Windows user presses and
 * `Backspace` is what the same key is called on a Mac keyboard (`trash` design
 * D12). Requiring a modifier for an action that cannot lose anything would only
 * make it slower.
 */
export const KEY_DELETE = 'Delete'
export const KEY_BACKSPACE = 'Backspace'

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
  { where: 'Anywhere', keys: ['F11'], action: 'Enter or leave full screen' },
  { where: 'Search field', keys: ['Esc'], action: 'Leave the field' },
  { where: 'Grid', keys: ['←', '→', '↑', '↓'], action: 'Move the focused card' },
  { where: 'Grid', keys: ['Home', 'End'], action: 'Focus the first or last card' },
  {
    where: 'Grid',
    keys: ['⇧ ←', '⇧ →', '⇧ ↑', '⇧ ↓'],
    action: 'Move the focus and select from the anchor to it',
  },
  { where: 'Grid', keys: ['⇧ Home', '⇧ End'], action: 'Select to the first or last image' },
  { where: 'Library', keys: ['⌘ A'], action: 'Select every image in the result' },
  { where: 'Library', keys: ['Esc'], action: 'Clear the selection' },
  { where: 'Grid', keys: ['Enter', 'Space'], action: 'Open the focused image' },
  {
    where: 'Grid',
    keys: ['Delete', 'Backspace'],
    action: 'Move the selection, or the focused image, to the trash',
  },
  { where: 'Grid', keys: ['I'], action: 'Show or hide the inspector' },
  { where: 'Viewer', keys: ['←', '→'], action: 'Previous or next image' },
  { where: 'Viewer', keys: ['↑', '↓'], action: 'The image one grid row up or down' },
  { where: 'Viewer', keys: ['I'], action: 'Show or hide the inspector' },
  { where: 'Viewer', keys: ['Esc', 'Space'], action: 'Close, focusing the image shown last' },
]

/**
 * The map's way out of a text field: blurs it and swallows the key, so the
 * screen-wide bindings are live again the moment it fires. Shared by both
 * search fields (`browse-feedback` design D3) — before this it was
 * `SearchBar`'s own `leaveOnEscape`, and the toolbar's free-text field would
 * otherwise have needed a second copy of the same three lines.
 */
export function blurOnEscape(event: KeyboardEvent): void {
  if (event.key !== KEY_ESCAPE) return
  event.preventDefault()
  if (event.currentTarget instanceof HTMLElement) event.currentTarget.blur()
}

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

/**
 * True while the event happened inside a dialog — the bulk tag dialog, a
 * confirmation, the viewer. A screen-wide binding has to stand down there: the
 * dialog is what the user is looking at, and a shortcut fired underneath it
 * would act on a grid they cannot see.
 *
 * Read from the target rather than from each dialog's open flag, so a dialog
 * added later is covered without the flag being threaded up to the screen. Every
 * shape in use is here: `<dialog>` for the viewer, bits-ui's `role="dialog"` for
 * what shadcn draws, `role="alertdialog"` for its alert dialog, and an open
 * menu, which is modal in the same sense — ⌘A under a context menu would
 * select the whole result behind it.
 */
export function isInDialog(event: Event): boolean {
  const target = event.target
  return target instanceof HTMLElement
    && target.closest('dialog, [role="dialog"], [role="alertdialog"], [role="menu"]') !== null
}

/**
 * True when this key press means "move to the trash" (`trash` design D12).
 *
 * In the trash view it is never true: binding permanent deletion to a key would
 * put the app's only unrecoverable action one keystroke from a grid the user
 * reached by clicking around, and no confirmation survives a key pressed
 * reflexively. The unmodified keys are acceptable in the library view precisely
 * because what they fire is the reversible half of the pair.
 */
export function isTrashKey(event: KeyboardEvent, view: 'library' | 'trash'): boolean {
  if (view !== 'library' || isTypingTarget(event)) return false
  return event.key === KEY_DELETE || event.key === KEY_BACKSPACE
}
