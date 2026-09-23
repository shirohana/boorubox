> One Sonnet unit, two commits in order: group 1 (Rust + shared + the settings store), then
> group 2 (the pure function and the sidebar). No migration, no library data. Design D1–D5
> decide every shape; do not re-decide them. Gate after each commit: `mise run check` green.
> No unit ticks a hand check. Files shared with changes in flight (`shared/src/index.ts`,
> `model.rs`): edit only the `AppSettings` blocks.

## 1. The stored key (`packages/app/src-tauri`, `packages/shared`, `packages/app/src/lib/api`)

- [x] 1.1 `settings.rs`: `HIDDEN_TAG_CATEGORIES = "hiddenTagCategories"`, the
      `hidden_tag_categories: Vec<TagCategory>` field (default empty), load per D1 (non-array →
      empty; each element parsed with `str::parse::<TagCategory>`, unknown or non-string
      dropped, repeats kept once), save as the `as_str` names, the `From<&Settings>` line, and
      `Settings::set_tag_category_hidden(category, hidden)` per D2. Tests: the existing
      `settings_round_trip_through_the_file` gains `hidden_tag_categories: vec![Artist, Meta]`;
      `a_file_without_the_hidden_tag_categories_key_hides_nothing`;
      `unknown_names_in_hidden_tag_categories_are_dropped_and_the_rest_kept` (store
      `["artist", "chartreuse", 3, "artist", "meta"]` → `[Artist, Meta]`);
      `a_hidden_tag_categories_value_that_is_not_a_list_reads_as_empty`;
      `hiding_a_category_twice_keeps_it_once_and_showing_it_removes_it` (the method, no store).
      Verify: `cargo test settings::` passes.
- [x] 1.2 `model.rs`: `AppSettings` gains `hidden_tag_categories: Vec<TagCategory>` and drops
      `Copy` (D1); `app_settings_crosses_the_wire_in_camel_case` gains
      `"hiddenTagCategories": ["artist"]` (clone the value if the test reuses it).
      `commands.rs`: `set_tag_category_hidden(category: TagCategory, hidden: bool, app, state)`
      through `write_settings`, doc comment naming this change's D2; registered in `lib.rs`.
      Test `the_hidden_categories_reach_the_state_and_the_store`, copied from
      `the_collections_fold_reaches_the_state_and_the_store`: hide artist, assert the answer,
      `app_settings` and `settings::load` all list it; show it again, all three empty. Verify:
      `cargo test commands::` passes, `mise run clippy` clean.
- [x] 1.3 `packages/shared/src/index.ts`: `hiddenTagCategories: TagCategory[]` on
      `AppSettings`, doc comment in the `collectionsCollapsed` style pointing at D1.
      `api/commands.ts`: `setTagCategoryHidden(category, hidden)` →
      `invoke('set_tag_category_hidden', { category, hidden })`; `api/settings.svelte.ts`:
      `settings.setTagCategoryHidden`. Every `AppSettings` literal in
      `api/commands.test.ts` and `api/settings.svelte.test.ts` gains the field. Tests:
      `set_tag_category_hidden passes the category and the flag and returns the settings`
      (commands.test.ts, the `set_collections_collapsed` test's shape); in
      `settings.svelte.test.ts`, `setTagCategoryHidden` replaces `current` with the answer.
      Verify: `mise run check` green; commit.

## 2. The filter and the toggles (`packages/app`), after group 1

- [x] 2.1 `domain/tag-categories.ts`: `sidebarRows(rows, nameOf, isActive, categoryOf, hidden)`
      per D3. Tests in `tag-categories.test.ts`, named after the spec's scenarios: `nothing
      hidden keeps today's order` (the `Order` scenario's five tags with `highres` active);
      `a hidden category's inactive rows are left out`; `a hidden category's active row stays,
      first` (`kantoku` artist, excluded, artist hidden); `an or-group term counts as active`
      (via an `isActive` built from `activeTerms('kantoku or 1girl')`); `every category hidden leaves only
      the active rows`; `every category hidden with nothing active is empty`. Verify:
      `pnpm --filter @boorubox/app test tag-categories` passes.
- [x] 2.2 `components/tags/categories.ts`: `CATEGORY_ICON` per D4, with a test in
      `categories.test.ts` that it names every `CATEGORY_ORDER` entry. `TagSidebar.svelte`:
      `hidden` and `rows` as the two `$derived` lines in D3 (the `rows` doc comment moves to
      `sidebarRows`, the component keeps one line pointing at it); the toggle row under the
      heading per D5; the two empty states per D3. No other component edited (design
      Non-Goals: Inspector, ImageCard footer, TagInput, pinned chips). Verify: `mise run check`
      green; `grep -rn hiddenTagCategories packages/app/src` lists only `TagSidebar.svelte`,
      the api files and their tests; commit.
      Hand check: the five icons sit under "Tags" in the order palette, ©, user, tag, info,
      each in its category's colour, at the size of a row's `+`, light and dark theme.
      Hand check: clicking the artist icon dims it and the artist rows leave the list; clicking
      it again brings them back in place; the grid and the inspector do not change.
      Hand check: with artist hidden, `-<an artist tag>` in the search lists that tag first,
      marked as excluded; its `−` takes it out of the query.
      Hand check: hide meta, quit and relaunch; the meta icon is still dimmed and meta tags are
      still out of the list.
      Hand check: hide all five with an empty search; the list says the tags are hidden, the
      five icons stay, and one click brings its category back.
      Hand check: a screen reader (VoiceOver) reads each icon as "Artist tags, toggle button,
      selected" and flips to not selected when hidden; the tooltip reads "Hide artist tags" /
      "Show artist tags".
