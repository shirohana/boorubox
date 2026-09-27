> Two units. Unit R (Rust + shared + the TS invoke wrappers, `packages/app/src-tauri`,
> `packages/shared`, `packages/app/src/lib/api`) runs first, Sonnet; unit U (webview,
> `packages/app/src`) runs after R has landed and after `tag-row-and-inspector-fixes`,
> `settings-pages` and `tag-notes` have landed (all four edit `Inspector.svelte`,
> `TagVocabularyMenuItems.svelte` or the Artists section), Sonnet. Retried on Opus if the gate
> fails; one Opus review of the whole change. Design D1–D9 decide every shape; do not
> re-decide them. No schema change. Gates: unit R `mise run check` (the whole gate — it owns
> Rust and removes a wrapper the webview still calls, so it also moves that one call site, 1.6);
> unit U `pnpm lint && pnpm typecheck && pnpm test`. No unit commits or ticks a hand check.

## 1. Unit R — preview, match, apply, commands (`packages/app/src-tauri`, `packages/shared`, `packages/app/src/lib/api`)

- [x] 1.1 `artists.rs`: extract `owning_entry(candidate, entries) -> Option<&ArtistEntry>`
      (longest owning URL) from `derive`, which keeps its category skip on top (D3). Tests:
      the existing `derive` tests pass unchanged; `owning_entry` picks the longest of two
      nested owners and answers `None` for the `08110` neighbour.
- [x] 1.2 `artists.rs`: `preview(conn, input: &ArtistPreviewInput) -> ArtistPreview` per D3,
      replacing `rename_preview`. Tests: `artist_preview_lists_the_entrys_urls_and_counts_trash`;
      `artist_preview_offers_an_unowned_candidate` (X handle, `https://x.com/<handle>`);
      `artist_preview_withholds_a_candidate_this_entry_owns`;
      `artist_preview_withholds_a_candidate_another_entry_owns`;
      `artist_preview_of_a_tag_without_entry_has_no_urls`; Pixiv record without `userId` → no
      candidate.
- [x] 1.3 `artists.rs`: `artist_match(conn, adapter) -> Option<ArtistMatch>` per D5. Tests:
      `artist_match_names_the_owner_and_the_derived_tag` (entry `metaljelly` owns
      `x.com/metaljelly0811`, handle `MetalJelly0811` → owner `metaljelly`, derived
      `metaljelly0811`, url `https://x.com/metaljelly0811`);
      `artist_match_without_owner_has_only_the_derived_tag`;
      `artist_match_of_a_pixiv_user` (`userId` `3439325`);
      `artist_match_is_none_without_a_profile_url` (Pixiv without `userId`, a Danbooru
      record).
- [x] 1.4 `artists.rs`: `from_profiles`, `apply_preview(conn, urls)`, `apply(library, tag)` per
      D6 — one transaction, `Conflict::Keep`, `mark_updated`, sidecars and `write_library`
      after the commit in `rename`'s order. Tests:
      `apply_tags_only_images_whose_profile_the_entry_owns` (`@alice_art` yes, `@alice_art2`
      no, a Danbooru capture no);
      `apply_is_additive` (other tags, the derived tag included, kept);
      `apply_twice_tags_nothing_the_second_time` (`tagged` then `skipped`);
      `apply_leaves_the_trash_alone`;
      `apply_rewrites_the_sidecars_of_the_images_it_tagged` (the sidecar lists the tag; an
      untouched image's sidecar is unchanged);
      `apply_moves_updated_at_of_tagged_images_only`;
      `apply_refuses_a_tag_without_entry` and `apply_refuses_a_tag_of_another_category`, each
      leaving every image unchanged;
      `apply_preview_counts_images_and_untagged` (trash excluded, an unnormalisable URL
      ignored).
- [x] 1.5 `model.rs` + `packages/shared/src/index.ts`: `ArtistPreviewInput`, `ArtistPreview`,
      `ArtistMatch`, `ArtistApplyPreview`, `ArtistApplyReport` added;
      `RenameArtistPreviewInput`, `RenameArtistPreview` removed. The wire test
      `artist_entry_and_rename_types_cross_the_wire_in_camel_case` is renamed
      `artist_types_cross_the_wire_in_camel_case` and covers every artist type left.
- [x] 1.6 `commands.rs` + `lib.rs`: `artist_preview`, `artist_match`, `artists_apply_preview`,
      `artists_apply` per D8, registered; `rename_artist_preview` removed. One commands test
      reaching the open library for `artists_apply` in the shape of the existing artist
      command test. `packages/app/src/lib/api/commands.ts`: `artistPreview`, `artistMatch`,
      `artistsApplyPreview`, `artistsApply` with one invoke test each in `commands.test.ts`;
      `renameArtistPreview` and its test removed, and its one caller
      (`RenameArtistDialog.svelte`) switched to `artistPreview({ tag: from, adapter })` reading
      `[...urls, candidate]` so the gate stays green — unit U rewrites that file anyway.
- [x] 1.7 `packages/app/src/lib/api/latest-only.ts`: `latestOnly()` per D5 (webview helper, but
      pure and needed by nothing else in R — written here so U starts from tested plumbing).
      `latest-only.test.ts`: a later call resolving first wins and the earlier answer comes
      back `{ current: false }`; a stale rejection is `{ current: false }` too; a current
      rejection rejects.
- [x] 1.8 `openspec/changes/archive/2026-09-25-artist-entries/design.md` D6: append a dated
      paragraph (2026-09-28) saying `artist-workflow` D1 reverses it and why, in two
      sentences. `ArtistsSection.svelte`'s FIXME is left for unit U (it closes it).
- [x] 1.9 Gate `mise run check` green. Handoff: exact command and wrapper names, the refusal
      wordings of `apply`, what `artist_match` answers for an image with no adapter record,
      and anything that deviated from D3–D8.

## 2. Unit U — the dialog, the menu item, the Artist row, Settings apply (`packages/app/src`)

- [x] 2.1 Before anything: re-read, as landed, `Inspector.svelte`, `TagVocabularyMenuItems.svelte`,
      `TagSidebar.svelte`, `LibraryScreen.svelte` and the Artists settings page and section
      (`settings-pages` moved it to `routes/settings/artists`; `tag-notes` added "Edit note…"
      and a note dialog). Where `tag-notes` gave its dialog a menu callback of a different
      shape than D2's `oneditartist`, follow that shape and say so in the handoff.
- [x] 2.2 `components/artists/artist-dialog.ts`: `confirmLabel`, `savePlan`, `canConfirm`
      per D4. `artist-dialog.test.ts`: edit same name → "Save" and `[upsert]`; edit same
      name with apply → `[upsert, apply(from)]`; edit new name → "Rename 120 images" and
      `[rename]`; "Rename 1 image" singular; a name differing only by spaces/underscores is
      the same name; create → "Create and tag 14 images", "Create artist" at 0 or unknown,
      `[upsert, apply]` with the URLs unioned into an existing entry of that name;
      `canConfirm` false for an untouched edit, while the preview is pending, while saving.
- [x] 2.3 `git mv components/tags/RenameArtistDialog.svelte
      components/artists/ArtistDialog.svelte`, rewritten per D4: `request` prop (edit / create),
      `artistPreview` on an edit open, `artistsList` on a create open, the carriers line, the
      apply line re-read through `artistsApplyPreview` 300 ms after the URL lines change via
      `latestOnly`, the "Apply to existing images" checkbox (unchecked on every open, shown
      while the lines are non-empty), the steps from `savePlan` run in order, the refusal
      line, `onsaved`. Keep the file's mount-unconditionally and `portalTo` comments.
- [x] 2.4 `TagVocabularyMenuItems.svelte`: the `oneditartist` prop and the "Edit artist…" item
      per D1/D2 (header comment updated: the item and why it needs a host). `Inspector.svelte`:
      remove the badge menu's "Rename artist…" item and its comment; `renamingArtist` becomes
      `editingArtist: ArtistDialogRequest | null` (same snapshot doc); pass `oneditartist` to
      the badge's and the pinned chip's `TagVocabularyMenuItems`; mount `ArtistDialog`;
      `onartistrenamed` renamed `onartistsaved` here and in `LibraryScreen.svelte`.
      `TagSidebar.svelte`: its own `ArtistDialog` mount (no adapter) and an `onartistsaved`
      prop, wired to `afterWrite` in `LibraryScreen.svelte`.
- [x] 2.5 `Inspector.svelte`: the Artist row per D5 — `inspectedId` derived, one effect on it
      and on `artistMatchGeneration`, `artistMatch` through `latestOnly`, the chip button
      toggling the owner with the badge's marking and artist colour, "Create artist…"
      opening the dialog in create mode, row directly above Account, same in both
      `editingFacts` states, a comment citing `artist-workflow` D5. Adopt `latestOnly` in the
      pinned-counts effect only if it drops in unchanged; else a FIXME there naming it.
- [x] 2.6 Artists section (as landed by `settings-pages`): "Apply…" per entry with the
      confirmation and result line per D7; delete the FIXME; replace the notice paragraph with
      D7's text; `vocabulary.refresh()` after an apply.
- [x] 2.7 Gate `pnpm lint && pnpm typecheck && pnpm test` green. Handoff: the callback shape
      used (D2's or `tag-notes`'), where each `ArtistDialog` is mounted, whether `pinnedFetch`
      adopted `latestOnly`, anything the smoke test should look at.
- [ ] 2.8 Hand check (owner): on the real vault, an X image whose account a maintained artist
      owns shows the Artist chip.
      Hand check: open an X-captured image whose handle's `https://x.com/<handle>` is listed
      under an artist in Settings → Artists; the facts read Title, Source, Artist, Account,
      Page, Image; the Artist row shows that artist's tag in the artist colour, and clicking it
      adds the tag to the search and clicking again removes it; the Account row still reads
      `@handle` and still toggles `account:`.
      Seen (lead's smoke 2026-09-28): using the entry that 2.10 created (iv70311741): the facts read Title, Source, Artist, Account, Page, Image, and the Artist row shows `iv70311741` in the artist colour (49-after-create-artist.png). A click added it to the search and a second click removed it (50-artist-chip-click-1.png, 51-artist-chip-click-2.png); Account reads `@IV70311741` and toggles `account:IV70311741` (52-account-click.png).
- [ ] 2.9 Hand check (owner): a Pixiv image with a maintained artist shows the chip.
      Hand check: open a Pixiv capture made with the `userId`-reading extension whose
      `https://www.pixiv.net/users/<id>` an artist owns; the Artist row shows that tag; an old
      Pixiv capture without `userId` shows no Artist row.
      Seen (lead's smoke 2026-09-28): an old Pixiv capture without `userId` shows no Artist row (68-pixiv-no-artist-row.png). Not seen: the `userId` half; test-1 has no such capture.
- [ ] 2.10 Hand check (owner): Create artist on an unowned X account tags its images.
      Hand check: open an X image whose account no entry owns; the Artist row offers "Create
      artist…"; the dialog prefills the handle and `https://x.com/<handle>` and says "N images
      come from this profile; M of them carry no artist tag"; confirm; the Artist row now shows
      the tag, the sidebar count for that tag is N (it moved by the M that had none), and
      Settings → Artists lists the entry.
      Seen (lead's smoke 2026-09-28): on an @IV70311741 image the Artist row offered "Create artist…". The dialog prefilled `iv70311741` and `https://x.com/iv70311741` (handle lowercased) and read "13 images come from these URLs; 13 of them carry no artist tag." (wording differs from this task's "from this profile") (48-create-artist-dialog.png). After confirming, the Artist row shows the tag, the sidebar count is 13 (up from 0) (49-after-create-artist.png), and Settings → Artists lists the entry (58-settings-artists-note.png).
      Seen (lead's re-smoke 2026-09-28, at deab612): the dialog now reads "13 images come from this profile; 13 of them carry no artist tag." (re-14-create-artist-dialog-prefilled.png). Typing `iv_art` with no `iv70311741` tag in the library shows no rename line and the button stays "Create and tag 13 images" (re-15-create-artist-iv_art-no-derived-tag.png). After tagging the image `artist:iv70311741` in the editor (re-16-tag-derived-artist-tag.png, re-17-derived-tag-saved.png), Create artist… with `iv_art` shows "“iv70311741” is renamed to “iv_art” on 1 image." above "13 images come from this profile; 12 of them carry no artist tag." and the button reads "Create and rename 1 image" (re-18-create-artist-rename-line.png). After confirming, the Artist row shows `iv_art`, tiles carry it (re-19-after-create-rename.png), the sidebar lists `iv_art` 13 and no `iv70311741` (re-20-sidebar-iv_art-no-handle-tag.png); the database has `iv_art` (artist) on 13 images and no `iv70311741` row.
- [ ] 2.11 Hand check (owner): Edit artist from a sidebar row adds a second X URL, and the next
      capture from it gets the tag.
      Hand check: right-click an artist tag in the sidebar, choose "Edit artist…", see the
      entry's URLs and the carrier count, add a second `https://x.com/<other>` line, the button
      reads "Save"; save with "Apply to existing images" left unchecked and see no image
      change; capture a post from `@<other>` with the extension and see it carry the artist's
      tag, and its Artist row show it.
      Seen (lead's smoke 2026-09-28): right-clicking `iv70311741` in the sidebar → "Edit artist…" shows its URL and "13 images carry" (53-artist-tag-menu.png, 54-edit-artist-dialog.png). After adding `https://x.com/hwa_sawa` the button reads "Save" and the apply line reads "14 images come from these URLs; 1 of them carry no artist tag." ("carry" should be "carries" for 1) (55-edit-artist-second-url.png). Saving with Apply unchecked changed no image (checked in the database), and the @hwa_sawa image's Artist row already shows `iv70311741` by URL match (57-hwa-sawa-artist-row.png). Not seen: the capture from @hwa_sawa; there is no extension.
      Seen (lead's re-smoke 2026-09-28, at deab612): sidebar right-click `iv_art` → "Edit artist…" (re-21-sidebar-artist-menu.png): Save is greyed out while nothing has changed (re-22-edit-artist-save-disabled.png) and enabled after adding `https://x.com/someone_else` (re-23-edit-artist-second-url.png). After Save the inspector's Artist row still shows `iv_art` (re-24-inspector-after-sidebar-save.png). Settings → Artists lists `iv_art` with both URLs (re-25-settings-artists-two-urls.png); the Apply control is a ▷ icon with the tooltip "Apply to stored images" (re-26-apply-button-tooltip.png), and it opens "Apply “iv_art”?" with "13 images come from these URLs; 0 of them carry no artist tag. Applying adds “iv_art” to all 13; no tag is removed." and "Apply to 13 images" (re-27-apply-confirm.png); Cancel closed it and nothing changed (re-28-after-apply-cancel.png).

## Handoff

- Lead (2026-09-28, before unit R starts): create mode with a name that differs from the
  derived tag, when the derived tag exists in the vocabulary, goes through `rename_artist`
  (from = derived, to = name, urls) first — it retags the handle's carriers and creates the
  entry in one transaction — and only then `artists_apply` for the profile's images that
  carry neither (a capture the `Conflict::Skip` rule left untagged). Leaving the handle tag
  beside the chosen name is what the owner corrects by hand today; the design's "Apply is
  additive" stays true for apply itself. Amend design D-whichever covers create mode to say
  this, with the argument, when unit U lands; `artist-dialog.ts`'s pure step function is
  where the branch lives and is tested.

- Unit R (2026-09-28, landed): all of 1.1–1.9 done, gate green (below). No deviation from
  D1–D9; no schema change (still `MIGRATIONS.len() == 13`, unchanged by this change).

  **Commands and wrappers** (all through `with_library_off_main_thread`, all in
  `packages/app/src-tauri/src/commands.rs` / `packages/app/src/lib/api/commands.ts`):
  - `artist_preview(input: ArtistPreviewInput) -> ArtistPreview` / `artistPreview(input)`.
  - `artist_match(adapter: SiteAdapterRecord) -> ArtistMatch | null` / `artistMatch(adapter)`
    — **takes the adapter directly, not wrapped**: the Rust parameter is bare `adapter`, so
    the wire body is `{ adapter }`, not `{ input: { adapter } }`.
  - `artists_apply_preview(urls: string[]) -> ArtistApplyPreview` / `artistsApplyPreview(urls)`.
  - `artists_apply(tag: string) -> ArtistApplyReport` / `artistsApply(tag)`.
  - `rename_artist_preview` / `renameArtistPreview` and `RenameArtistPreviewInput` /
    `RenameArtistPreview` are gone. `artists_list`, `artists_upsert`, `artists_delete`,
    `rename_artist` are unchanged.

  **Wire types** (`model.rs` + `packages/shared/src/index.ts`, camelCase, mirrored by hand):
  - `ArtistPreviewInput { tag: string, adapter: SiteAdapterRecord | null }`.
  - `ArtistPreview { carriers: number, urls: string[], candidate: string | null }` — `urls`
    is the tag's entry's own URLs (each already `https://…`), empty for a tag with no entry;
    `candidate` is the image's profile URL (`https://…`) or `null` — no adapter, no profile
    URL, or **any** entry (this tag's own or another's) already owns it. The dialog's textarea
    is `urls` then, only when present, `candidate` appended as one more line.
  - `ArtistMatch { url: string, owner: string | null, derived: string | null }` — `owner` is
    the tag of the entry that owns `url` (category not consulted — a chip can name a tag that
    was since recategorised; that is Settings → Artists' problem, not this call's). `derived`
    is the artist tag a capture from this record would derive, independent of any entry —
    **it is not a claim that tag exists in the vocabulary.** To answer "does the derived name
    already exist as a tag" (needed for the create-mode branch the lead's note above
    describes), ask the vocabulary store (`vocabulary.categoryOf(derived)` — the same call
    `TagVocabularyMenuItems` already makes) rather than `artist_match`: a second existence
    check inside `artist_match` would duplicate what the vocabulary store already answers for
    every other tag-shaped question in the webview.
  - `ArtistApplyPreview { images: number, untagged: number }`,
    `ArtistApplyReport { tagged: number, skipped: number }`.

  **`apply` refusal wordings** (`AppError::NotFound`/`BadRequest`, shown verbatim by
  `errorText`): no entry — `` `artist entry {tag}` `` (i.e. the message is
  `"artist entry alice not found"` (`AppError::NotFound` displays as `{0} not found`) for `apply(library, "alice")` with no matching `artist_urls` row);
  wrong category — `tags::category_conflict`'s wording, the same "…is a general tag and
  cannot become an artist tag; use another name, e.g. {tag}_(artist)" reused by `upsert` and
  `rename`.

  **`latestOnly()`** (`packages/app/src/lib/api/latest-only.ts`, exported from `$lib/api`):
  `latestOnly<T>(): (promise: Promise<T>) => Promise<{ current: true, value: T } | { current:
  false }>` — call it once per effect/dialog to get a ticket function, then call that function
  with each fetch's promise; only the still-current call's `{ current: true, value }` should be
  acted on, a stale resolution or rejection both come back `{ current: false }`, and a
  current rejection rethrows (catch it same as any other call).

  **`RenameArtistDialog.svelte`**: its one `renameArtistPreview` call is now
  `artistPreview({ tag: from, adapter })`, and `urlsText` is built from
  `[...preview.urls, ...(preview.candidate ? [preview.candidate] : [])]` — this is the
  gate-keeping edit only; the file is otherwise untouched and unit U rewrites it wholesale
  into `components/artists/ArtistDialog.svelte` per D4.

  **Gate**: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
  (825 passed), `pnpm --filter @boorubox/shared test`, `pnpm --filter @boorubox/app
  typecheck`, `pnpm --filter @boorubox/app exec vitest run src/lib/api` (286 passed),
  `pnpm lint` — all green. Full `mise run check` not run (per brief).

- Unit U (2026-09-28, landed): 2.1–2.7 done. Gate green: `pnpm lint`, `pnpm typecheck`
  (0 errors, 1238 files), `pnpm -r test` (shared 1, extension 96, app 803 — includes the new
  `artist-dialog.test.ts`'s 20). Hand checks 2.8–2.11 left for the owner, unticked.

  **The callback shape used**: D2's own, unchanged by `tag-notes` — `TagVocabularyMenuItems`
  gained `oneditartist?: (tag: string) => void`, optional (unlike `oneditnote`, required),
  rendering the "Edit artist…" item only when given and `vocabulary.categoryOf(name) ===
  'artist'`, first among the vocabulary items, before the Danbooru look-up. `tag-notes`'
  Handoff had flagged that this shared menu has no image in scope and asked whether "Edit
  artist…" would need its own image-bearing prop threaded everywhere `oneditnote` is — it
  doesn't: D3's `artist_preview` already dropped the image requirement (the entry's own URLs
  come from the tag alone), so the callback needs only the tag name, exactly like
  `oneditnote`. The per-image "Rename artist…" item that lived on `Inspector.svelte`'s own
  tag-chip menu (outside `TagVocabularyMenuItems`, `artist-entries` D6) is gone; that menu now
  mounts `TagVocabularyMenuItems` with `oneditartist` like every other mount.

  **Where each `ArtistDialog` is mounted**: `Inspector.svelte` mounts one, unconditionally,
  `{portalTo}`, for both the tag badge's menu and the pinned chip's menu — each builds its own
  `{ mode: 'edit', tag, adapter: image?.adapter ?? null }` snapshot (the chip has an image only
  while one image is inspected; over a selection `image` is the panel's own `$derived`, which
  is `null` then, so the snapshot opens with no candidate, exactly `artist-entries` D6's old
  rule for the chip). `TagSidebar.svelte` mounts its own, no `portalTo` (never inside the
  viewer), always `adapter: null`. Create mode opens only from `Inspector.svelte`'s new Artist
  row's "Create artist…" — `{ mode: 'create', tag: derived ?? '', url }` from `artistMatch`'s
  own answer.

  **`pinnedFetch` did not adopt `latestOnly`.** Left as it was, with a note rather than a
  silent pass: it carries `pinnedCountsKnown`/`pinnedTagCounts`/`pinnedCollectionCounts`
  together as one guarded fetch keyed on a generation counter already (see its own doc
  comment), and retrofitting `latestOnly` there would still need the same generation-keyed
  `$effect` gate around it (the fetch must not even start unless the key or the generations
  moved) — `latestOnly` only replaces the ticket-vs-ticket comparison inside the async body,
  which that effect already has correctly. Swapping it in would touch a working, reviewed
  effect for no behaviour change. `latestOnly` was used fresh instead for the two genuinely new
  races this unit added: the Artist row's `artist_match` fetch (keyed on `inspectedId`) and
  `ArtistDialog`'s own apply-preview re-read (debounced 300 ms on the URL lines).

  **The Artist row's own effect**: keyed on `inspectedId = $derived(imageId ?? null)` — reusing
  the existing `imageId` derived (not a second `image?.id` derivation) — and on
  `artistMatchGeneration`, a counter `ArtistDialog`'s `onsaved` bumps. `image.adapter` is read
  through `untrack` inside the effect body, the same rule flagged twice in review this run
  (`vocabulary.noteOf`/`categoryOf`, `library.status?.x`): `image` is reassigned wholesale on
  every `results.refresh()`, so a tracked read of `image.adapter` would refetch on every
  capture landing mid-view even at the same id. The row is absent (not a placeholder) whenever
  `artistMatchAnswer` is `null`, whether that is "pending" or "no profile URL at all" — both
  read the same in the template, which is what design D5 asks for.

  **Deviations / calls made beyond the letter of the brief**:
  - The lead's create-mode branch (rename before create when the derived tag already exists as
    an artist tag) lives in `artist-dialog.ts`'s `savePlan`, gated on a `derivedExists: boolean`
    the caller computes as `vocabulary.categoryOf(request.tag) === 'artist'` at save time (a
    plain synchronous read in the `save()` handler, not inside an `$effect`, so the
    wholesale-reassignment rule above does not apply to it). `design.md` D4 is amended in place
    with the argument; `spec.md` was left untouched — the brief named only `design.md`, and the
    existing "Create under another name" scenario's precondition (the *target* name already
    has an entry) is a different condition from this branch's (the *derived* name already
    exists as an artist tag), so nothing there was made incorrect, only incomplete. Flagging
    for the reviewer: a spec scenario for this branch may still be worth adding.
  - `ConfirmDialog.svelte` (not in this run's file ownership) gained one optional prop,
    `confirmDisabled` (default `false`), backward-compatible with its seven other callers,
    because D7 requires the Settings → Artists apply confirmation's button disabled at 0
    images and writing a second confirmation dialog from scratch to get one disabled button
    would have duplicated the existing one. Its header comment (naming the bar for an eighth
    caller) is updated to record Apply as that eighth, on its own ground.
  - `LibraryScreen.svelte` (not in this run's file ownership either) had its two
    `onartistrenamed={afterWrite}` call sites (the `Inspector` and `Lightbox` mounts) and its
    `TagSidebar` mount renamed/extended to match the prop renames tasks.md 2.4 specifies for
    `Inspector.svelte`, plus a new `onartistsaved={afterWrite}` for `TagSidebar`'s own dialog —
    required for typecheck to pass once the props those components require changed shape.
    `Lightbox.svelte` itself needed the same rename (`onartistrenamed` → `onartistsaved`) since
    it only forwards the prop to its own `Inspector` mount.
  - `commands.ts`/`vocabulary.svelte.ts` were not touched, per the brief; no wrapper was found
    missing.

  **Reviewer**: `artist-dialog.ts`'s `savePlan` (the lead's branch) and its tests; `ArtistDialog.svelte`'s
  two-mode `$effect` (edit fetches `artistPreview`, create fetches `artistsList`) and the
  debounced apply-preview effect; `Inspector.svelte`'s new Artist-row effect
  (`inspectedId`/`untrack`) and its three `TagVocabularyMenuItems`/`oneditartist` call sites;
  `ArtistsSection.svelte`'s apply flow and its `ConfirmDialog` extension.

- Review of unit U and smoke 2026-09-28 (applied, uncommitted at hand-off): `oneditartist`
  is now required (D2 amended). `underscored` in `artist-dialog.ts` mirrors
  `tags::underscored` and backs `sameName` and every name a step carries. The edit baseline
  is the stored URLs only, so an appended candidate is itself a confirmable edit. The shared
  `artistRevision` (`api/artists.svelte.ts`) replaces the inspector-local counter: bumped by
  each landed dialog step and by Settings → Artists' save, delete and Apply; the Artist row
  keys on it through a `$derived`. `ArtistDialog`'s `onsaved` now only refreshes (once per
  attempt, if a step landed); closing is `onclose`; a retry runs only the unlanded steps.
  Create that renames the derived tag says so ("`derived` is renamed to `name` on N images")
  and labels "Create and rename N images" (D4, spec "Create under another name renames the
  derived tag"). `Inspector.svelte`'s editor-reset effect hands focus back through
  `onrelease` when the closed editor held it (`tag-row-and-inspector-fixes`); the note glyph
  is `shrink-0` and the inspector's tag badge `inline-flex whitespace-nowrap` (`tag-notes`).
