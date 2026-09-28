## Why

The owner wants Danbooru's `source:` metatag (2026-09-28): `source:https://x.com/*`,
`source:https://*fanbox.cc/*`. Danbooru's rule (`Post.source_matches` in the local clone):
case-insensitive, the value is a prefix unless `*` says otherwise, `*` is a wildcard, `none`
means an empty source. The app already has a definition of an image's source — the value the
upload dialog prefills as Source, the page URL and failing that the image URL
(`upload-form.ts`) — so the filter reads that value and nothing new. Requirements §6 (the
query language).

## What Changes

- **`source:<pattern>` and `-source:<pattern>`** in the tag query: prefix match on the
  image's source URL, `*` a wildcard, case ignored; `source:none` matches images with no
  source, `-source:none` images with one. Excluding a pattern keeps images with no source, as
  `-account:` keeps pages naming no account.
- **The image record carries `sourceUrl`**, computed by Rust from the one SQL expression the
  filter matches on, and the upload dialog prefills Source from it instead of choosing between
  two fields itself.
- The stamp grammar and the tag editor's completion learn the new metatag as a search-only
  word.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `library-browse`: the `source:` metatag.
- `booru-upload`: where the Source prefill comes from.

## Non-goals

- A quoted value with spaces (`source:"a b"`): URLs have none.
- A comma list (`source:a,b`): a URL may contain a comma; two terms AND, as Danbooru's do.
- `source:` as a stamp edit, or toggles for it in the sidebar or the inspector.
- Matching the adapter record's `postUrl`: the source is the stored page URL, as for upload.
