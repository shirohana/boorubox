## Context

`library.rs` owns the folder layout and already defines the image path in one place
(`LibraryPaths::image_path`); `thumbs.rs` defines the thumbnail path in one place too. The
webview composes the full-image path itself in `src/lib/api/assets.ts` — the one place the
layout is spelled twice, and the reason this change touches `ImageRecord`. Every other reader
(`maintenance`, `export`, `trash`, `thumbs`, the booru upload) goes through `image_path`.

`legacy-bundle-import` lands after this change and imports thousands of images through the
same `ingest::store_image` door, so it inherits the layout with no work of its own.

## Goals / Non-Goals

**Goals:**

- No folder in a library ever holds more than a small fraction of the images.
- A library made before this change opens and works unchanged from the user's point of view.
- The layout is still defined in exactly one place per file kind.

**Non-Goals:**

- Validating ids (see proposal). Changing the id format. Content-addressed storage.

## Decisions

**D1. Two levels from the id, Danbooru's shape.** `images/<a1>/<b2>/<id>.<ext>` and
`.thumbs/<a1>/<b2>/<id>.jpg`, `a1` = `id[0..2]`, `b2` = `id[2..4]`. Ids are UUID v4 strings
everywhere real (the extension, local import, the legacy bundle), so the hex spreads evenly
over 65,536 buckets. One level (256 buckets) would carry 25,000 images today; two levels cost
nothing more and never need a second reshaping. The function is tolerant, not validating: an
id shorter than four characters uses what it has (`a` → `images/a/a.png`, `abc` →
`images/ab/c/abc.png`) — the existing tests use ids like `a` and `id-1`, and a shard function
that refuses them would move validation into a path helper, where it does not belong. The
capture door does not validate ids at all yet; a `FIXME` in `library.rs` names that as the
right place.

**D2. The relative path is a string built with `/`, and `ImageRecord` carries it.** One
function, `LibraryPaths::relative_image_path(id, ext) -> String`, produces
`images/a1/b2/<id>.<ext>`; `image_path` is `root.join(that)`. `row_to_record` puts the same
string in `ImageRecord.file`, and `assets.ts` does `convertFileSrc(`${libraryPath}/${image.file}`)`.
Forward slashes on every platform: the webview has no path module, Windows accepts `/` inside
an absolute path, and `Path::join` on Windows accepts a `/`-separated relative path. The
alternative — a shared TypeScript `shardPath(id)` mirroring the Rust one — is exactly the
two-runtimes-drift the project rules forbid.

**D3. Buckets are created by the writer, on demand.** `ingest::write_through_inbox` and the
thumbnail writer call `create_dir_all` on the destination's parent before the rename. Creating
65,536 directories up front would make an empty library a 130,000-entry tree. `create_dir_all`
on an existing directory is one `stat`.

**D4. Relayout runs in `Library::open_or_create`, beside the inbox sweep.** For `images/`
and `.thumbs/`: every regular file directly in the directory (not in a bucket) is renamed to
the path the shard function gives for its stem (`<id>.<ext>`; a thumbnail's `.jpg.part` is
removed like a stray inbox part). Nothing under a bucket is touched, so a library already in
shape costs one `read_dir` of two near-empty directories. No schema version is involved:
the database does not describe the layout and never did. Libraries that predate this exist
only on the owner's machines, but a copy of one is exactly what the drive-app probe opens.

**D5. Permanent delete leaves the bucket.** `delete_forever` unlinks the file and the
thumbnail as today. An empty `a1/b2/` directory is harmless and rare; removing it means a
race with a concurrent write into the same bucket for no visible gain.

## Risks / Trade-offs

- [A tool that lists `images/` flat] → nothing in the repo does; the export writes zip
  entries by id and tags, not by path.
- [The asset protocol scope] → `open_into_state` allows the library directory recursively
  (`allow_directory(&directory, true)`), so nested paths already pass.
- [An old library on a read-only volume] → the relayout's rename fails and the open fails with
  the IO error, which the start screen shows. Acceptable: the library was not writable anyway.

## Open Questions

None.
