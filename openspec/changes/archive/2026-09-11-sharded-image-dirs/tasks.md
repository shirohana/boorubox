> One implementing agent (**A**) owns everything here; it is one Rust unit with one small
> webview edit whose only consumer is `assets.ts`. Lands before `legacy-bundle-import`'s Rust
> group, which edits `ingest.rs` too.

## 1. Sharded layout

- [x] 1.1 `packages/app/src-tauri/src/library.rs`: `LibraryPaths::relative_image_path(id, ext)
      -> String` producing `images/<a1>/<b2>/<id>.<ext>` per design D1/D2 (tolerant of ids
      shorter than four characters), `image_path` built on it, and a `FIXME` naming id
      validation at the capture door as the missing check; verify tests for a UUID id, a
      one-character id and a three-character id, and that `image_path` is `root` joined with
      the relative path.

      Note (agent A, review fix): `image_path` no longer builds on `relative_image_path` —
      joining a `/`-string onto a `root` made of native separators leaves a mixed-separator
      path in a user-visible report on Windows. Both now push `shard_dirs`' components
      directly; `relative_image_path`'s `/`-string is solely `ImageRecord.file`'s wire format.
      `shard_dirs` also moved from a byte slice to `char_indices`-based slicing (design D1
      says "characters", and a byte slice panics on a non-ASCII id at a char boundary — hit
      on every stem during relayout, not just the one bad id); a non-ASCII test covers it.
- [x] 1.2 `packages/app/src-tauri/src/thumbs.rs`: `thumbnail_path` and its `.part` sibling
      under `.thumbs/<a1>/<b2>/`, sharing the shard function with 1.1 (one definition of the
      bucket rule); the thumbnail writer creates the bucket before its rename (design D3);
      verify the existing thumbs tests pass with the new path and one asserts the bucket
      directory was created.
- [x] 1.3 `packages/app/src-tauri/src/ingest.rs`: `write_through_inbox` creates the
      destination's parent before the rename (design D3); `row_to_record` fills the new
      `ImageRecord.file` (`model.rs`, serde `file`) from `relative_image_path`; verify a
      stored image's file is at the bucketed path, and its record's `file` names it.
- [x] 1.4 `packages/app/src-tauri/src/library.rs`: relayout on open (design D4) for `images/`
      and `.thumbs/` — flat regular files moved into buckets, stray `.jpg.part` under
      `.thumbs/` removed, files already in buckets untouched; verify tests that a flat image
      file and a flat thumbnail move to their bucketed paths and the record still loads, that
      a second open moves nothing (compare the tree), and that a bucketed file is left alone.

      Note (agent A, review fix): a per-entry failure (a Dropbox "conflicted copy", an iCloud
      placeholder, or a stray file exactly named the two-character bucket prefix an id needs —
      which fails `create_dir_all` outright) no longer aborts the whole open with `?`; it is
      skipped, and every other entry still migrates. A rename that would overwrite an
      already-bucketed file (a conflicted copy restored flat, a library caught mid-migration)
      is skipped too rather than destroying it. The thumbs closure now also refuses a flat
      file whose extension is not `jpg`, rather than renaming it onto one it never had. Three
      tests cover these; this is a deliberate narrowing of design D4's "a read-only volume
      fails the open" risk note — every failure is now swallowed, read-only volume included,
      since a `?` on the first bad entry was worse than a library that opens with some files
      left flat.
- [x] 1.5 `packages/shared/src/index.ts`: `ImageRecord.file: string` with a doc comment
      naming it as the path relative to the library root, `/`-separated;
      `packages/app/src/lib/api/assets.ts`: `imageUrl` uses `image.file`; every test or
      fixture constructing an `ImageRecord` gains the field; verify `mise run typecheck` and
      `pnpm -r test` pass.
- [x] 1.6 Every Rust test that asserted or removed a file under `images/` or `.thumbs/` by a
      flat path (`maintenance.rs`, `export.rs`, `trash.rs`, `thumbs.rs`, `ingest.rs`,
      `library.rs`) goes through `image_path` / `thumbnail_path`; verify `cargo test` green
      and `grep -rn 'join(format!("{id}' src-tauri/src | grep -v '/library.rs:'` finds
      nothing (superseding the original `images_dir().join|thumbs_dir().join` clause: once
      `image_path` stopped joining a `/`-string in the 1.1 review fix, that grep would pass
      trivially whether or not a flat path had crept back in; this one matches the actual
      shape a reintroduced flat path takes — `<dir>.join(format!("{id}...` — anywhere outside
      `library.rs`, whose own `part_path` legitimately builds `inbox/`'s flat name that way).

      Note (agent A): also fixed `commands.rs`'s
      `search_reports_a_file_that_vanished_since_it_was_imported`, which built a flat path by
      hand (`library.path().join("images").join(...)`) — not in this task's listed files, but
      broken by the same layout change and now routed through `LibraryPaths::image_path`.

      Note (agent A, review fix): `ingest.rs`'s `the_same_id_twice_stores_one_file_and_one_row`
      and `thumbs.rs`'s `a_missing_source_file_is_an_error_not_a_panic` counted directory
      entries one level deep, which a bucket satisfies at "one entry" regardless of how many
      files sit inside it. Now `image_path(...).is_file()` plus a recursive file count, and
      `!part_path(...).exists()`, respectively.
- [x] 1.7 `openspec/specs/library-folder/spec.md` is updated by the archive; the drive-app
      skill and any doc naming `images/<id>.<ext>` are amended to the bucketed path; verify
      `grep -rn 'images/<id>\|\.thumbs/<id>' . | grep -v -E 'openspec/changes/(archive|
      sharded-image-dirs)|openspec/specs/'` finds nothing (widened from `--include=*.md` to
      the whole repo — a Rust or TypeScript doc comment is not a `.md` file and was missing
      from the original clause; `openspec/changes/sharded-image-dirs` joins the exclusion
      alongside `archive` and `specs/` on the owner's word that this change's own planning
      prose, quoting the old path as the thing being changed, and the main spec, rewritten by
      the archive, are both fine to leave as they read).

      Note (agent A, review fix): fixed two live-code doc comments the `--include=*.md` grep
      never saw: `model.rs`'s `ImageRecord.file` doc (named the old flat shape in an aside) and
      `commands.ts`'s `thumbnailPath` doc (still said `<library>/.thumbs/<id>.jpg` outright).
      `docs/requirements.md` fixed to `images/<a1>/<b2>/<id>.<ext>` with the reason (folder
      listings and cloud sync degrading past tens of thousands of files in one folder, not
      merely "hex chars", which was never true of an id's shard). The drive-app skill names no
      layout path. The widened grep now passes.
