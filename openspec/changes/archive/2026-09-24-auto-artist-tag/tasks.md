> One Sonnet unit, Rust only (`packages/app/src-tauri`) plus one comment in the webview; no
> migration, no shared-type or wire change. Assumes every archived change has landed; runs in
> parallel with `tag-category-visibility` (webview) and before `pinned-collections`, which
> edits `Inspector.svelte` in other regions (task 1.5 touches only the Account row's comment).
> Design D1–D6 decide every shape; do not re-decide them. Gate: `mise run check` green. No
> unit ticks a hand check.

## 1. Unit A — the derived artist tag (`packages/app/src-tauri`, one comment in `packages/app`)

- [x] 1.1 `tags.rs` + `collections.rs`: `tags::underscored(text)` per D1 (`canonical`, then
      whitespace runs → `_`); `collections::slug` calls it, its doc comment kept. Tests in
      `tags.rs`: `underscored_lower_cases_and_joins_whitespace_runs_with_underscores`
      (`"Some  Artist"` → `"some_artist"`), `underscored_reads_an_ideographic_space_as_whitespace`
      (`"山田\u{3000}太郎"` → `"山田_太郎"`), `underscored_of_a_blank_text_is_empty`. The two
      existing `slug_*` tests in `collections.rs` stay and pass unchanged. Verify:
      `cargo test tags:: collections::` pass.
- [x] 1.2 `tags.rs`: `Conflict::Skip` per D2, its doc comment naming the third caller. Tests:
      `skip_links_nothing_when_the_name_exists_under_another_category` (general `cat`, then
      `artist:cat` under Skip → the image carries no `cat`, `cat` still general, answer
      `false`), `skip_leaves_a_character_tag_unlinked_and_unchanged`,
      `skip_creates_a_missing_tag_under_its_category` (answer `true`),
      `skip_links_a_tag_already_under_the_same_category` (answer `false`). Verify:
      `cargo test tags::` pass.
- [x] 1.3 `rules.rs`: `artist_tag(adapter)` per D1's table. Tests beside the `haystacks`
      ones: `an_x_record_derives_its_handle_as_the_artist_name` (`Alice_Art` → `alice_art`;
      `displayName` ignored), `a_pixiv_record_derives_its_display_name_spelled_as_a_tag`
      (`"Some  Artist"` → `some_artist`), `a_record_from_another_site_derives_no_artist`
      (`site: "danbooru"` carrying `artist`), `a_record_missing_the_field_derives_no_artist`,
      `a_non_text_or_blank_field_derives_no_artist` (array, number, `"   "`). Verify:
      `cargo test rules::` pass.
- [x] 1.4 `ingest.rs`: `resolve_tag_text` answers `ResolvedTags { text, artist }` per D3,
      inside the existing non-bundle `if`; `insert_rows` links `artist` with `Conflict::Skip`
      after the rules' `Keep` link, before the commit, OR-ing its answer into `categorised`.
      Tests (spec `capture-ingest`, "A capture is stored with its author as an artist tag"):
      `an_x_capture_is_stored_with_its_handle_as_an_artist_tag`,
      `a_pixiv_capture_is_stored_with_its_display_name_as_an_artist_tag`,
      `a_capture_from_another_site_gains_no_artist_tag`,
      `a_capture_whose_record_lacks_the_field_gains_no_artist_tag`,
      `a_capture_whose_artist_name_is_a_general_tag_is_stored_without_it`,
      `a_capture_whose_artist_name_is_a_character_tag_is_stored_without_it`,
      `a_capture_whose_artist_name_is_already_an_artist_carries_it`,
      `a_rule_naming_the_artist_name_plainly_in_the_same_capture_keeps_it_general`,
      `a_bundle_sourced_ingest_with_an_adapter_record_gains_no_artist_tag` (source
      `LegacyBundle`, adapter set by hand — proves the gate, not the importer's `None`),
      `re_delivering_a_stored_id_gains_no_artist_tag`,
      `a_capture_creating_an_artist_tag_rewrites_library_json` (the tag is in
      `library.json`'s `tags` list after the call). Verify: `cargo test ingest::` pass.
- [x] 1.5 `http/captures.rs` tests + `Inspector.svelte` comment. The existing
      `a_rule_matching_the_adapter_record_tags_the_capture` now also carries `alice` (artist):
      update its expected `tags` to what the record reads back (ordered by name) and say why in
      its doc comment. New: `an_x_capture_answers_201_carrying_its_artist_tag` and
      `a_capture_whose_handle_is_a_general_tag_still_answers_201_without_it` (spec "The answer
      never waits on the tag"). `packages/app/src/lib/components/library/Inspector.svelte`:
      the Account row's comment rewritten per D6, markup untouched. Verify: `mise run check`
      green.
- [ ] 1.6 Hand check: with the real extension, capture an image from an X post and one from a
      Pixiv artwork whose author's name has a space or a capital; each arrives with the
      author as an artist tag (artist colour, spelled lower-case with `_`); capture from X
      again by an author whose handle is already a general tag in the library — the capture
      succeeds and the image carries no such tag; the X image's inspector shows the Account
      row and the artist tag side by side.
