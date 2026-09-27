## Context

- `components/tags/FilterRow.svelte` is the one row both sidebar lists draw
  (`sidebar-inspector-polish` D7): `TagSidebar.svelte` (~:128-140) and
  `CollectionsSection.svelte` (~:201-221). Its trigger `div` is `flex items-center gap-1`, so
  the include button, the exclude button, the name button and the count are all 4px apart; each
  icon button is `p-0.5` around a `size-3` lucide icon. The file's header says no behaviour
  lives in the row — every action is the caller's.
- `domain/danbooru.ts` `danbooruLookup(name, category) -> { label, url }` (`menu-polish` D2):
  the wiki for any category but artist, the artist search for an artist.
  `TagVocabularyMenuItems.svelte` (~:52-62) opens it with `void openExternal(lookup.url)` under
  a FIXME that drops the failure string — no notice surface exists. `api/opener.ts`
  `openExternal(url) -> Promise<string | null>` resolves with the reason or `null`.
- `components/library/Inspector.svelte`: the title row (~:888-903) holds the only facts pencil
  (`startEditFacts`, ~:329, which focuses the Title input so the form scrolls into view). The
  facts `<dl>` (~:1195) has no heading; every other section (Rating, Tags, Collections,
  Posted) opens with an `h3` in `text-xs font-medium text-muted-foreground`, and Tags puts its
  pencil at the right of that `h3` row.
- Inspector's tag editor (~:245-285): `draft`, `error`, `editingTags`; the reseed effect
  returns early while `editingTags`, so nothing closes the editor when `image` changes, and
  `save` → `write` (~:395) sends `draft` to `results.saveTags(image.id, …)` — the live image.
  The facts form already closes on a new id (~:316-326), reading `image?.id` inside its effect.
  Both placements (`LibraryScreen.svelte`, `Lightbox.svelte`) keep one Inspector mounted across
  images; `e` opens the editor with `void tick().then(() => inspector?.startEditTags())`, right
  after the panel mounts.
- No test in `packages/app` mounts a Svelte component (`artist-entries` handoff, confirmed
  2026-09-28): the `*.svelte.test.ts` files test rune stores, under `// @vitest-environment
  jsdom` where they touch the DOM (`fullscreen.svelte.test.ts`, which also shows the
  `mockIPC`/`clearMocks` shape from `@tauri-apps/api/mocks`). Vitest 5, vite 8,
  vite-plugin-svelte 7; `vite.config.ts` has no `test` or `resolve` block.

## Goals / Non-Goals

**Goals:** the four asks as the proposal states them, with the wrong-image write proven closed
by a component test.

**Non-Goals:** a notice surface for a refused look-up; component tests for panels this change
does not touch; any change to the context menus' own items.

## Decisions

**D1. The row's buttons are one `flex` group with `gap-0`.** In `FilterRow.svelte` the `?`
(when present), include and exclude buttons move into a `<span class="flex shrink-0
items-center">`; the row keeps `gap-1`, which now separates the group from the name and the
name from the count. `gap-0`, not `gap-px`: each button's own `p-0.5` already puts 4px between
two glyphs, which is the tight pair the owner asked for, and with no gap the hit areas abut —
a `gap-px` would leave a 1px strip between them where a click lands on the row's context-menu
trigger and does nothing. The group is `shrink-0` so a long name truncates, never the buttons.

**D2. `FilterRow` takes an optional `lookup` and stays behaviour-free.** New prop
`lookup?: { label: string, open: () => void }`. Present → a `?` button renders first in the D1
group; absent → nothing is drawn (spec `app-frame`, a control that does nothing is not shown).
The row never imports anything Danbooru: it is shared with collections, and its header's rule
(every action is the caller's) holds — `CollectionsSection.svelte` passes nothing and is not
edited. `TagSidebar.svelte` passes, per row, `lookup={tagLookup(name)}`, a local function
building `{ label, open }` from `danbooruLookup(name, vocabulary.categoryOf(name))` — the
category read fresh per render as `nameClass` beside it already is (`tag-vocabulary` D5).
One object rather than `lookupLabel` + `onlookup`: two props that are only meaningful together
can be passed half.

**D3. The glyph is a literal `?` in a `size-3` box.** The button carries the same classes as
its neighbours (`shrink-0 rounded-sm p-0.5 text-muted-foreground hover:text-foreground`); its
content is `<span class="flex size-3 items-center justify-center text-xs leading-none
font-medium">?</span>`, so the box is exactly the icons' 12px and the row's height does not
move (`text-xs` alone would bring a 16px line-height). `title` is the look-up's label ("Open
Danbooru wiki" / "Search artist on Danbooru"); `aria-label` is `"{label}: {name}"`, since the
include and exclude buttons beside it name the tag too and a list of identical labels is
unreadable to a screen reader. It is a `<button type="button">` like the others: tab order is
`?`, include, exclude, name — nothing extra to wire.

**D4. One function opens a look-up and drops its failure, under the one FIXME.** A new
`components/tags/danbooru-open.ts` exports `openDanbooruLookup(lookup: DanbooruLookup): void`,
which is `void openExternal(lookup.url)`. The FIXME now in `TagVocabularyMenuItems.svelte`
moves onto it, reworded to cover both callers: the URL is always `https:`, never one of
`ExternalLink`'s named failures, and the right shape is a shared transient-notice surface the
frame does not have yet (the menu has closed; the row has no line to show it in). The menu
item and `TagSidebar`'s `tagLookup` both call it. Same logic twice would be the same FIXME
twice, and the one that is fixed first leaves the other behind.

**D5. The second pencil sits in a heading row over the facts, labelled "Details".** The facts
`<dl>` is wrapped in a `section` shaped like the Tags section: `border-t border-border px-4
py-3`, a first row `flex items-center justify-between gap-2` with `<h3 class="text-xs
font-medium text-muted-foreground">Details</h3>` and the pencil `Button` (`size="icon-xs"
variant="ghost"`, `aria-label` and `title` "Edit title and addresses", `onclick=
{startEditFacts}`, inside `{#if !editingFacts}`), then the `<dl>` with its border and padding
moved to the section and `mt-2` for the gap the other `h3`s get from `mb-2`. A heading row over
the first row on its right: the `<dl>` is a two-column grid whose Title value wraps, and a
pencil in that cell would sit mid-text or push the title narrower; every other section of the
panel already opens with an `h3` and Tags puts its pencil exactly there, so the facts gain the
same shape rather than a third one. "Details" because the block has no name on screen today and
"Facts" is the spec's word, not the owner's — the owner may rename it at the hand check; the
title row's pencil and its handler are unchanged, and both call the same `startEditFacts`, whose
Title-focus is what scrolls the form into view from either door.

**D6. One effect, keyed on a derived id, closes both editors when the image changes.**
`const imageId = $derived(image?.id)`; one `$effect` reads `imageId` and, when it differs from
the id the effect last saw, sets `editingTags = false`, `error = null`, `editingFacts = false`,
`factsError = null`. It replaces the facts form's effect (~:316-326), which read `image?.id`
off an object reassigned wholesale on every write — the CLAUDE.md `$effect` rule. `$derived`
compares by value, so a new record for the same image (a save, a rating, a capture refreshing
it; `updatedAt` moved) leaves `imageId` equal and the effect does not run. The first run after
mount only records the id: `e` opens the editor through `tick()` right after the panel mounts,
and a first run that reset would race it. The reseed effect is untouched — once `editingTags`
drops it runs for the new image and seeds its draft. Closing, not reseeding in place: the typed
text belongs to the image it was typed for (proposal non-goals), and an editor that silently
swapped its text under the caret would be the same surprise inverted. `write` needs no guard of
its own: with the editor closed on the change, no save button exists to send the old draft, and
an in-flight save already read `image.id` before its `await`.

**D7. The test mounts `Inspector.svelte` itself.** `components/library/Inspector.svelte.test.ts`,
`// @vitest-environment jsdom`, the first component mount in the package. It mounts with
`mount(Inspector, { target, props })` from `svelte`, `props` a `$state` object (the file is a
`.svelte.test.ts`, so runes compile) holding `image`, a `results` stub cast to `SearchResults`
whose `saveTags`/`saveFacts`/`saveRating` are `vi.fn()`, an `actions` stub and `tagQuery: ''`;
`mockIPC` answers every store read the panel makes at mount with an empty value. The editor is
opened through the component's exported `startEditTags` (the path `e` takes), the textarea is
typed into with an `input` event, and the image is swapped by assigning `props.image` then
`flushSync()`. If `mount` throws that it is unavailable on the server, Svelte resolved its
server build: add Svelte's documented recipe to `vite.config.ts`,
`resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined`, and nothing else. A
harness need beyond that — a production-code change made only so the panel mounts — is out of
scope: stop and write it in the handoff rather than substituting a weaker test. A pure-helper
test would prove `!==` and not the wiring, which is where the bug was.

## Risks / Trade-offs

- [The first component mount pulls in bits-ui, the vocabulary and collections stores and
  `UploadAction`] → `mockIPC` with a catch-all answer covers the store reads; D7 bounds the
  scope and names the stop condition.
- [A browser resolve condition under Vitest changes resolution for every existing test] → it
  is Svelte's own recipe for component tests; the existing store tests run under it in the
  same gate, and a failure there is visible, not silent.
- [A draft is lost on an accidental tile click] → accepted (proposal non-goals); the loss is a
  few typed tags, where the bug it replaces wrote them onto the wrong image.
- ["Details" is a new visible word] → called out at the hand check.
