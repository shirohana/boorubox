> Three units, serial — every one edits `components/tags/PinnedTagsPanel.svelte` and its test,
> so no two run at once. Each is Sonnet, retried on Opus if the gate fails, and Opus-reviewed
> before the next starts. Design D1–D9 decide every shape. Gate for each: `pnpm lint && pnpm
> typecheck && pnpm --filter @boorubox/app test` from the repo root. No unit commits, runs the
> dev server or ticks a hand check.

## 1. Unit M — modes, fold, one header, no scroll (`packages/app/src/lib/components/tags`)

Owns `PinnedTagsPanel.svelte`, `PinnedTagsPanel.svelte.test.ts`, `PinnedTagsDialog.svelte`,
`PinnedTagsDialog.svelte.test.ts`.

- [x] 1.1 `PinnedTagsPanel.svelte`: `startEditing` prop and the `editing` state per D1; the
      Edit toggle first in the bar; leaving edit mode closes the pin field, the naming form,
      the ticks and the drafts; the move `Select` and "New group…" render only while editing.
      The component's header comment gains the two modes and which door starts in which.
- [x] 1.2 Read mode per D2: label-only header, plain rows (coloured name text, no handle,
      checkbox, Unpin button, "+", arrows or name field), the row's `ContextMenu.Trigger`
      kept, "No tags" kept.
- [x] 1.3 One header per position per D3: the header snippet rendered once inside one
      `ContextMenu.Root`, `ContextMenu.Trigger disabled={!editing || group.tags.length > 0}`,
      "Delete group" in the content as today, a comment on the `Root` saying why the trigger
      is disabled rather than the menu conditional.
- [x] 1.4 The fold per D4: the toggle button leading the header in both modes with
      `aria-expanded` and the Fold/Unfold label, writing `setGroupCollapsed` through
      `write()`; a folded section renders no list, no "No tags", no pin field (folding the
      adding group closes it) and shows `· <count>`; it stays a drop target.
- [x] 1.5 `write()` focuses with `{ preventScroll: true }` in both branches (D5), with the
      comment saying why a restore never scrolls.
- [x] 1.6 `PinnedTagsDialog.svelte` passes `startEditing`.
- [x] 1.7 Tests (D9, first eight items): read mount has no field, checkbox or Unpin and
      offers Edit; Edit shows them, a second press hides them and clears a tick;
      `startEditing` mounts editing; the fold writes and hides; Move down on a full group
      beside an empty one keeps `document.activeElement` the same node; focus is called with
      `preventScroll`; the dialog test asserts the panel is editing. Existing tests that mount
      the panel reading and expect edit controls pass `startEditing: true` — a test's intent
      is the control it drives, not the mode.
- [x] 1.8 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.9 Hand check (owner): Settings → Pinned tags opens with no field and no checkbox;
      Edit shows them, Edit again hides them; the dialog from "Manage pinned tags…" opens
      editing; fold a group on the page — the strip's row folds too; with a group taller
      than the dialog, scroll down and press Move down on a group next to an empty one — the
      groups do not jump to the top, in the dialog and on the page.
      Seen (lead, on screen 2026-10-04, macOS WebKit, scratch copy of the 10-01 smoke lib): Settings → Pinned tags opened reading, no field and no checkbox, index at the top, Edit in the bar (boorubox-vault/smoke-2026-10-04/02-s.png); Edit showed fields, checkboxes, handles and Unpin (boorubox-vault/smoke-2026-10-04/05-s.png), Edit again hid them (boorubox-vault/smoke-2026-10-04/13-s.png); the dialog from a chip's "Manage pinned tags…" opened editing (boorubox-vault/smoke-2026-10-04/16-s.png); folding Games on the page wrote `collapsed=1` and the inspector strip read "Games · 7" (boorubox-vault/smoke-2026-10-04/04-s.png, boorubox-vault/smoke-2026-10-04/14-s.png); with the first group scrolled out of view, Move down on Style beside the empty Spare swapped them with no jump to the top (boorubox-vault/smoke-2026-10-04/06-s.png → boorubox-vault/smoke-2026-10-04/07-s.png); the view shifted up by about five rows on that first move at the page's maximum scroll and not on the Move up that followed (boorubox-vault/smoke-2026-10-04/08-s.png) — a scroll-anchoring adjustment, not the focus jump. The folded count wrapped onto two lines beside the name field in edit mode (fixed after the smoke: `shrink-0 whitespace-nowrap`, boorubox-vault/smoke-2026-10-04/22-count-s.png).

## 2. Unit N — the tag's menu and its note on the row (`packages/app/src/lib/components/tags`)

Owns `PinnedTagsPanel.svelte`, `PinnedTagsPanel.svelte.test.ts`,
`TagVocabularyMenuItems.svelte`. Starts after unit M is reviewed and committed.

- [x] 2.1 `TagVocabularyMenuItems.svelte`: `onmanagegroups: (() => void) | null`; the item is
      not offered on `null`; the prop's doc comment says `null` means the host is the manager.
- [x] 2.2 `PinnedTagsPanel.svelte`: the row's `ContextMenu.Content` mounts
      `TagVocabularyMenuItems` with `onmanagegroups={null}` and the row's own Unpin item goes
      (D6); the panel mounts one `TagNoteDialog` and one `ArtistDialog` unconditionally with
      `portalTo` — the `TagSidebar.svelte` shape for `editingNote`, `editingArtist` and the
      request literal; read `ArtistDialog.svelte`'s doc comment for what `onsaved` must do
      and do that. The Unpin icon button stays in edit mode.
- [x] 2.3 The note per D7 in both modes: the span after the name with the whole note in
      `title`, the name `min-w-0 truncate` without `flex-1`.
- [x] 2.4 Tests (D9): a right-click on a row offers "Open Danbooru wiki", "Edit note…", "New
      group above", "Move to Spare" and no "Manage pinned tags…"; "Edit note…" opens a dialog
      in the document; a noted tag's row shows the note text with the whole note in `title`,
      an unnoted one shows no span. Seed a note on one tag in the test's `seed()`.
- [x] 2.5 Gate green. Handoff.
- [ ] 2.6 Hand check (owner): right-click a row in the dialog — the menu reads as the
      sidebar's; Edit note… opens over the dialog, Escape closes the note dialog and leaves
      the panel open, a second Escape closes the panel; the note shows on the row in both
      modes and the whole note on hover; Open Danbooru wiki opens the browser.
      Seen (lead, on screen 2026-10-04, macOS WebKit, scratch copy of the 10-01 smoke lib): the row menu on `repost` read Edit note…, Open Danbooru wiki, Unpin, New group above/below, Move to #1/Games/Clothes/Pose/Spare, Category, and no Manage pinned tags…; the checkbox stayed unticked (boorubox-vault/smoke-2026-10-04/10-s.png); Edit note… opened the Tag note dialog over the page (boorubox-vault/smoke-2026-10-04/11-s.png), Save wrote the note and the row read it as dim text in edit and read mode (boorubox-vault/smoke-2026-10-04/12-s.png, boorubox-vault/smoke-2026-10-04/13-s.png). Not seen: Escape order inside the dialog door, Open Danbooru wiki, where focus lands after Unpin from the menu.

## 3. Unit O — the group index (`packages/app/src/lib/components/tags`, `routes/settings/pinned-tags`)

Owns `PinnedTagsPanel.svelte`, `PinnedTagsPanel.svelte.test.ts`,
`routes/settings/pinned-tags/+page.svelte`. Starts after unit N is reviewed and committed.

- [x] 3.1 `PinnedTagsPanel.svelte`: the `nav` per D8 above the groups region when more than
      one group exists, one ghost button per group reading `labelOf`, scrolling its section
      with `scrollIntoView({ block: 'start' })`; the prop `stickyBar` renamed `sticky` with
      D8's doc comment, the index `sticky top-0 z-10 bg-background pb-2` under it.
- [x] 3.2 `routes/settings/pinned-tags/+page.svelte` passes `sticky`.
- [x] 3.3 Tests (D9): no index at one group; `Animals`, `#2`, `Spare` at three; a click calls
      `scrollIntoView` on the right section (stub `Element.prototype.scrollIntoView`);
      `sticky` puts the sticky class on both the index and the bar.
- [x] 3.4 Gate green. Handoff.
- [ ] 3.5 Hand check (owner): on the Settings page with eight groups, the index stays at the
      top while the groups scroll and the bar at the bottom; clicking `Clothes` scrolls that
      group under the index; in the dialog the index sits under the header and a click
      scrolls the groups region.
      Seen (lead, on screen 2026-10-04, macOS WebKit, scratch copy of the 10-01 smoke lib): six groups: on the page the index stuck to the top of the scroller and the bar to the bottom while the groups scrolled; clicking Spare scrolled the page to its end (boorubox-vault/smoke-2026-10-04/03-s.png); in the dialog the index sat under the header and clicking Spare scrolled the groups region to it (boorubox-vault/smoke-2026-10-04/17-s.png). Eight groups and a wrapped index not seen.

## Handoff

(Each unit appends what the next needs to know here.)

Unit M: the header snippet now holds, in order, the fold button (read mode: label inside it,
`size="xs"`; edit mode: icon only, `icon-xs`), the name `Input` (edit only), the
`· <count>` span when folded, and the edit-only arrows and "+". One `ContextMenu.Root` per
section wraps it. The bar is: Edit toggle, the edit-only Move `Select`, a `mr-auto` spacer
span, then edit-only "New group…", then `barEnd`. The row is `{#if editing}` handle/checkbox/name button/Unpin `{:else}` a plain
coloured name `span` (`min-w-0 flex-1 truncate`); unit N adds the note span and swaps the
menu content in both branches' shared `ContextMenu.Root`. Tests mount editing by default
(`setup(refuse, answer, props = { startEditing: true })`); `reading()` mounts read mode.

Unit M review: the read-mode header had a `flex-1` spacer between the label and the
`· <count>`, which pushed the count to the far right; the spacer is gone, so the count follows
the label as D4 says. The move test now seeds a truly empty group (`setupBesideEmpty()`) and
checks both directions; it fails against a header rendered in two branches. bits-ui 2.19's
`ContextMenu.Trigger disabled` is real: it spreads `disabled` and `data-disabled` onto the
header `div` and returns early from `oncontextmenu` without `preventDefault`. Unit N: the row's
`ContextMenu.Root` wraps both mode branches, so the menu swap is one edit; the row's `span`
(read) and name `button` (edit) both carry `min-w-0 flex-1 truncate`, and D7 drops `flex-1`
from both.

Unit N: the row's menu is `TagVocabularyMenuItems` with `onmanagegroups={null}`; `TagNoteDialog`
and `ArtistDialog` mount at the panel's end with `portalTo`. `ArtistDialog`'s `onsaved` asks
the host to refresh vocabulary and search; the panel calls `vocabulary.refresh()` only (no
search handle), commented in place. The note is a shared `note(tag)` snippet rendered after
the name in both branches (`flex-1` moved to it); the edit-mode Unpin button carries `ml-auto`
so it stays at the row's end when a tag has no note. Unit O: the header and row markup are
otherwise unchanged; the note tests set `vocabulary.entries` inside the test, not in `seed()`.

Unit N review: the row menu's moves ("New group above/below", "Move to …", Unpin) write the
store directly, not through `write()`, so a renumbering left `adding`, `drafts` and
`renameErrors` on the wrong groups (an open pin field would pin into the new group). The
clear now hangs on `$effect.pre` over a derived `groupCount`, whoever wrote; `write()` only
restores focus. Test: "closes the pin field when a row menu move renumbers the groups". The
menu's Unpin gets no focus restore: at `onSelect` the focus is in the portalled menu, not the
panel, so `write()` would not restore it either; bits-ui returns it to the trigger only when
tabbable (the row `div` is not). Accepted; the 2.6 hand check watches where it lands. Unit O:
`ArtistDialog`'s `onsaved` stays `vocabulary.refresh()` (the dialog's commands answer artist
entries, never the vocabulary); the search behind the dialog door is not re-run.

Unit O: the index is `nav[aria-label="Groups"][data-group-index]` as the first child of the
panel root, `size="xs"` ghost buttons; `showGroup` queries `[data-group-position]` in the
groups region. Smoke test: on the page the nav's `sticky top-0` sticks to the layout's
scroller, so check it does not sit under a page header or clip the first group; with many
groups the wrapped index grows tall and eats the page height while sticky.
