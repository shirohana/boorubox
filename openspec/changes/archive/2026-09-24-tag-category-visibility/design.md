## Context

See proposal.md — Why. The code this change edits, as read on 2026-09-23 at `0a9dccc`:

- `TagSidebar.svelte` takes `tags: TagCount[] | null` (Rust's `tag_counts`, whole result, zero
  rows for the query's own tags), derives `terms = activeTerms(tagQuery)` and builds `rows` in
  a `$derived.by`: the rows the search includes or excludes first, then the rest, each half
  through `groupByCategory(part, row => row.name, vocabulary.categoryOf)`. `activeTerms`'s
  `included` already folds in the or-groups. An empty `rows` prints "No tags in these results".
  The section is `<h2>Tags</h2>` then the list.
- A category is known only to the webview's vocabulary store: `vocabulary.categoryOf(name)`
  (general when the name has no vocabulary row). `TagCount` carries no category.
- `CATEGORY_ORDER` and `groupByCategory` are in `domain/tag-categories.ts` (tests in
  `tag-categories.test.ts`); `CATEGORY_TEXT_CLASS` in `components/tags/categories.ts`.
- Preferences: `AppSettings` in `packages/shared/src/index.ts` (flat booleans and numbers),
  mirrored by `model::AppSettings` (derives `Copy`), filled `From<&Settings>` in `settings.rs`,
  where each field has a store key, a default, a per-field-fallback load and a save.
  `collectionsCollapsed` is the latest copy of the pattern (`browse-feedback` D4):
  `set_collections_collapsed` in `commands.rs` through `write_settings`, registered in
  `lib.rs`, wrapped in `api/commands.ts`, a method on the `settings` store, read in
  `CollectionsSection.svelte` as `$derived(... settings.current?.collectionsCollapsed ...)`.
  `settings.rs` already stores one list — `recentLibraries`, a JSON array of strings.
- No spec capability owns `AppSettings`. Each preference is specified inside the feature that
  reads it (`collections`: "kept with the app's other preferences, so the choice survives a
  restart"); this change does the same in `tag-sidebar`, and the key's spelling is specified
  here, in D1.
- `RatingPills.svelte` is the sidebar's toggle precedent: plain `<button aria-pressed>` with
  a coloured and a dimmed class, not the shadcn `Toggle`.

## Goals / Non-Goals

**Goals:**
- One stored list, validated on read; one setter; the filter as a pure, tested function.
- The row carries the category colours and reads at the sidebar's `text-xs` scale.

**Non-Goals:**
- Touching `Inspector.svelte`, `ImageCard.svelte`'s footer, `TagInput.svelte`,
  `pinned-state.ts` or the vocabulary store: none of them reads the new key, and none gains a
  parameter for it.
- Any Rust change beyond the settings key and its command: `tag_counts` and `query.rs` are
  untouched.

## Decisions

### D1. One key: `hiddenTagCategories`, a list of category names

`AppSettings.hiddenTagCategories: TagCategory[]` (shared), `hidden_tag_categories:
Vec<TagCategory>` on `model::AppSettings` and on `settings::Settings`, stored under the key
`HIDDEN_TAG_CATEGORIES = "hiddenTagCategories"` as a JSON array of the category names
(`TagCategory::as_str`, the one spelling of a category on the wire, in SQLite and now in the
file). Default: empty — nothing hidden, the list as it is today.

- *A list, not five booleans*: five keys are five loads, five saves, five fields, five tests
  and five chances for one to drift, for one fact — "which categories are hidden". The list
  also keeps the shape when a category is ever added: nothing hidden by default, no new key.
- *Stored names, not indices*: a hand-edited `settings.json` stays readable, and
  `CATEGORY_ORDER` can change without changing what the file means.
- *Load validates per element*: a non-array value reads as the default; inside an array, each
  element that is not a string naming a category (`str::parse::<TagCategory>`) is dropped, and
  a repeated name is kept once. Per element rather than all-or-nothing, the same reasoning as
  the file's per-field fallback: one stray entry a later build wrote must not un-hide the
  others. Order carries no meaning (the webview reads it as a set); load keeps the file's order.
- `model::AppSettings` loses `Copy` (a `Vec` cannot be `Copy`); it keeps `Clone`. Every
  producer goes through `AppSettings::from(&Settings)`, so the only fallout is a test that
  moves a value it uses again, which clones instead.

### D2. The command sets one category: `set_tag_category_hidden(category, hidden)`

`set_tag_category_hidden(category: TagCategory, hidden: bool) -> AppSettings` through
`write_settings`: hiding adds the category if it is absent, showing removes it. The
`Settings` side is a method, `Settings::set_tag_category_hidden`, so the dedupe rule is
written once and unit-tested without a store; `api/commands.ts` gains
`setTagCategoryHidden(category, hidden)` and the store `settings.setTagCategoryHidden`.

Rejected: `set_hidden_tag_categories(list)`, the whole list per write. The webview would
build the next list from its copy of `settings.current`, and two quick clicks on two toggles
before the first answer lands would each send a list missing the other's change. Per category
the write is idempotent and independent, and the list logic lives in Rust, next to the load
that validates the same list. A `TagCategory` argument deserialises through the enum's own
serde, so an unknown name is refused at the IPC boundary, never stored.

### D3. The filter is a pure function that owns the list's whole order

The two-half order moves out of the component into `domain/tag-categories.ts`:

```ts
export function sidebarRows<T>(
  rows: T[],
  nameOf: (row: T) => string,
  isActive: (name: string) => boolean,
  categoryOf: (name: string) => TagCategory,
  hidden: ReadonlySet<TagCategory>,
): T[]
```

It returns the active rows grouped by `groupByCategory`, then the inactive rows whose category
is not in `hidden`, grouped the same way. The hidden set applies to the second half only — that
placement *is* the rule "a tag the search uses stays visible", so it lives where a test can hold
it; left in the component, the rule that matters most would be the one with no test. Moving the
existing ordering with it is what makes that test possible: the halves and the filter are one
rule about the list. Generic over the row like `groupByCategory`, so it takes `TagCount`
without importing it into `domain/`.

In `TagSidebar.svelte`, per CLAUDE.md's rule for fields of a store object reassigned
wholesale:

```ts
const hidden = $derived(new Set(settings.current?.hiddenTagCategories ?? []))
const rows = $derived(tags && sidebarRows(tags, (row) => row.name, isActive, vocabulary.categoryOf, hidden))
```

No `$effect` anywhere: `rows` is a pure derivation of the props, the terms, the vocabulary and
`hidden`. A settings write for an unrelated field rebuilds `hidden` and so `rows`; writes are
single clicks, and the list is a few hundred rows at most — not worth a comparison to skip it.

The empty states split in two: `tags.length === 0` keeps "No tags in these results"; `tags` has
rows but `rows` is empty prints "Every tag here is in a hidden category" — the spec's
"says that the tags here are hidden, not that the results have none".

### D4. The glyphs: one lucide icon per category, in its colour

| Category  | Icon (`@lucide/svelte/icons/…`) | Why that one                                   |
|-----------|---------------------------------|------------------------------------------------|
| artist    | `palette`                       | the maker of the picture                       |
| copyright | `copyright`                     | the © sign is the category's own name           |
| character | `user`                          | a person in the picture                        |
| general   | `tag`                           | the plain tag, what most rows are              |
| meta      | `info`                          | facts about the file (`highres`, `translated`) |

Each at `size-3`, the size of the row's `+` and `−`, in `CATEGORY_TEXT_CLASS[category]`. The
map is `CATEGORY_ICON: Record<TagCategory, Component>` in `components/tags/categories.ts`,
beside the colours: it is styling, and `domain/` stays free of component imports.

Rejected: letters. Initials collide — copyright and character are both C — and the unique
two-letter forms (Ar, Co, Ch, Ge, Me) at `text-xs` in a 12 px square are five tiny words the eye
has to read, where an icon is recognised by shape. The colour is the legend either way; the
glyph is the second cue for a reader who cannot tell red from green, and a shape does that job
better than a letter pair. Rejected: coloured dots — colour alone fails that same reader.

### D5. The row: plain buttons, pressed means shown

Under `<h2>Tags</h2>`, a `<div role="group" aria-label="Tag categories" class="flex gap-0.5 px-1
pb-1">` with one `<button type="button">` per `CATEGORY_ORDER` entry, the `RatingPills`
precedent rather than the shadcn `Toggle`: the sidebar's other toggles are plain buttons with
`aria-pressed`, and the copy-in `Toggle`'s sizing is built for toolbars.

- `aria-label="{categoryLabel(category)} tags"` — constant, the WAI-ARIA toggle-button rule:
  the label names the thing, the pressed state carries the state.
- `aria-pressed={!hidden.has(category)}` — pressed is "shown", so the default row is five
  pressed buttons and a hidden category is the odd one out, as it is on screen.
- `title="Hide {label} tags"` / `"Show {label} tags"` — the pointer user's cue, which may say
  what a click will do.
- Hidden: `opacity-40` on the button (the dimming the owner asked for), full strength
  otherwise; `hover:bg-sidebar-accent` in both states, `rounded-sm p-0.5` as the row's buttons.
- The click calls `settings.setTagCategoryHidden(category, !hidden.has(category))`. A rejected
  write leaves `settings.current` as it was, so the button shows the stored state; the error
  goes to the console as the other fold toggles' do (no toast: a toggle that did not toggle is
  its own report).

The row stays drawn while `tags` is `null` (a search running) and when every category is
hidden, so there is always a way back.

### D6. One unit, two commits

The Rust key, the shared type, the command wrappers and the settings store are one commit:
adding a required field to `AppSettings` breaks every TS literal of it and the Rust mirror at
once, so no smaller slice builds. The pure function and the sidebar are the second commit, on
top. One Sonnet agent does both in order; the gate runs after each.

## Risks / Trade-offs

- [The owner hides general, forgets, and wonders where the tags went] → the dimmed toggle
  sits right under the heading, and the empty state says the tags are hidden.
- [A hidden category's tag in the search reappears in the list, which might read as the
  toggle not working] → it is drawn in the search marking at the top, the same place every
  searched tag is; the spec states it as the rule.
- [Siblings in flight edit the same files: `category-count-search` (`shared/src/index.ts`
  `ParsedTagSearch`, `model.rs` query types), `pinned-collections` (`shared` `Collection`,
  `model.rs`)] → different declarations in each file; a rebase conflict there is a union of
  separate blocks, handled by the run's conflict rule.
- [`categoryOf` answers general for a name the vocabulary has not loaded yet] → the same
  answer the row's colour already uses; a general-hidden sidebar may show an artist tag as
  general for the instant before the vocabulary answers, which is no worse than today's colour.
