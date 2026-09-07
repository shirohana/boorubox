// The one context-menu entry, named so it cannot be mistaken for the legacy
// extension's while both are installed during the migration (§3, design D12).
// The legacy entry reads "Save to Image Storage" under its own id; the two
// extensions have different ids, so the entries are independent and only the
// label tells them apart.

export const MENU_ID = 'boorubox-save-image'
export const MENU_TITLE = 'Save to BooruBox'

export function registerContextMenu() {
  chrome.contextMenus.create({
    id: MENU_ID,
    title: MENU_TITLE,
    contexts: ['image'],
  })
}
