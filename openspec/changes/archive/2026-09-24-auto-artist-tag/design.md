## Context

See proposal.md — Why. What the code holds today (all under `packages/app/src-tauri/src`):

- `http/captures.rs` `store_now` hands the whole adapter record to `ingest::store_image` as
  `IngestInput.adapter` and reads nothing out of it; its comment says reading a field is
  policy, and "policy lives in the app's rules, not in the door every image enters by".
- `ingest::insert_rows` runs one transaction: `resolve_tag_text` builds the candidate list —
  `input.tags`, then, unless `source == LegacyBundle`, `rules::auto_tags` over
  `rules::haystacks(page_title, adapter)` — and reads it with `tags::read_metatags`; the
  result is linked by one `tags::link_tags(&tx, id, &text, Conflict::Keep)`, whose `bool`
  ("a categorised row was born") makes `store_image` rewrite `library.json`.
- `tags::link_one_tag` settles a named category against the stored one: none → create under
  it; same → link; different → `Conflict::Refuse` errors (editor, bulk, stamps), `Keep` links
  the existing tag as it is (rules). It links through `link_tag` in every non-error branch.
- `tags::canonical` is `trim().to_lowercase()` and nothing more: it does not turn spaces into
  `_`. Editor text never needs it (tokens are split on whitespace before they reach Rust),
  which is why no tag writer has done it so far. `collections::slug` is the one Rust function
  that does: trim, lower-case, every run of whitespace → one `_` (`split_whitespace`, so
  U+3000, the ideographic space in Japanese names, counts).
- The adapters (`packages/extension/src/adapters/`): X sends `handle` (the permalink's path
  segment, no `@`), `displayName`, `postUrl`, `postText`, `originalUrl`; Pixiv sends `artist`
  (the display name, from the artist block in `main`), `workId`, `title`, `originalUrl`. Either
  may omit any field (`site-adapters`, "A failing adapter never blocks a capture"). In Rust
  `SiteAdapterRecord { site: String, fields: serde_json::Value }`.
- Only `http/captures.rs` passes an adapter record; local import and the bundle importer pass
  `None`.

## Goals / Non-Goals

**Goals:** the derivation is one pure function with its own tests, so the site-to-field table
is read in one place; the rules keep exactly the conflict behaviour they have; nothing a
capture could carry makes its answer anything but what it would be without the tag.

**Non-Goals:** a data patch or an "apply to existing" command; a setting; a user id; any
extension, shared-type or wire change; changing `rules::run` / `apply_rules_to_image` (they
re-run rules over stored images and do not derive the artist tag).

## Decisions

### D1. The derivation is `rules::artist_tag`, a table of site and field

`rules::artist_tag(adapter: &SiteAdapterRecord) -> Option<String>`, pure, beside `haystacks`:

| `adapter.site` | field read | why this field |
|---|---|---|
| `x` | `handle` | the name `account:` already searches and the Account row shows (D4) |
| `pixiv` | `artist` | the only author field the Pixiv adapter extracts (D4) |

Anything else → `None`. The field must be a JSON string; an array, a number or an absent key
→ `None`. The string is spelled by `tags::underscored` (below); an empty result → `None`. The
answer is the spelled name, not a token.

`tags::underscored(text) -> String` is `collections::slug`'s body, lifted: `canonical(text)`
then `split_whitespace().join("_")`. `collections::slug` becomes a call to it with its own doc
comment kept (its promise is about collections: `collection:` compiles against it). One
spelling for "free text becomes a name", two callers; a second copy in `rules.rs` would be the
drift CLAUDE.md's zero-duplication rule is for. `canonical` is not widened to do this: every
existing caller hands it a token that already has no inner whitespace, and widening it would
silently change what a rebuild or a stamp does with a name that somehow has one.

*Why in `rules.rs` and not `ingest.rs`:* the door stays policy-free (`captures.rs`'s comment
names the rules as where policy lives), and `rules.rs` is already the module that reads an
adapter record's fields. *Why a table and not `match` arms over a field per site:* the next
adapter with an author field is one row. *Why not the X `displayName`:* it is free text with
emoji and changes at will; the handle is ASCII, unique, and is already the account the app
searches.

### D2. `Conflict::Skip`: a third policy in the same enum

`Conflict { Refuse, Keep, Skip }`. In `link_one_tag`'s disagreement arm, `Skip` returns
`Ok(false)` before `link_tag`: the image is not linked to the existing tag, the row is not
touched. No existing row → created under the category and linked, as for every policy; same
category → linked. `Skip` only decides the disagreement, so a token with no category behaves
as it does under any policy (the derived token always has one). The `Conflict` doc comment
gains the third caller: the derived artist tag, a name the app guessed rather than one a
person or a rule wrote, which must never land an image under a general or character tag
because an author happens to share its name.

*Why a variant and not a separate call:* `link_one_tag` already owns "read the existing
category, decide, link"; a second function would repeat the read and the create, and the
enum's doc comment is where the three policies are compared side by side. *Why not `Keep`:*
the owner's decision — an author named `Cat` must not tag every capture `cat` (general);
leaving it off keeps the image in `arttags:0` where it gets a hand-picked `cat_(artist)`.

### D3. It joins in `resolve_tag_text` and links after the rules, in the same transaction

`resolve_tag_text` answers `ResolvedTags { text: tags::TagText, artist: Option<tags::TagText> }`.
Inside the existing `if input.source != ImageSource::LegacyBundle` block, beside the rules:
`input.adapter.and_then(rules::artist_tag)` → `tags::read_metatags(&[format!("artist:{name}")])`.
Through `read_metatags` rather than a hand-built `TagText`, because it is the one reader that
turns a token into a name and a category, and a hand-built one is a second place that must know
`TagText`'s shape. The bundle exemption is the same `if` the rules sit in: one gate, one
argument (`legacy-bundle-import`: what the bundle carries is what is stored).

`insert_rows` links `text` with `Conflict::Keep` exactly as today, then, when `artist` is
`Some`, `categorised |= tags::link_tags(&tx, id, &artist, Conflict::Skip)?`, before
`tx.commit()`. Same transaction: the image never exists without the tag it will have, and the
sidecar and `library.json` written after the commit already describe it.

*Why after the rules and not before:* the order decides the one case where both touch the
same name in one capture. Rules first means a rule's plain `alice` creates `alice` general and
the derived tag is skipped; artist first would create `alice` as an artist and the rule's
plain token would link it under Keep. The rule is text the owner wrote, the derived tag is a
guess, so the owner's text decides — and the outcome is the same one a later capture gets,
when the rule's general `alice` already exists before it arrives. *Why not append the token
to the rules' candidate list:* then it would be linked under `Keep`, which is the rules'
policy and the wrong one here.

### D4. The display name, not a user id — the owner's answer to a renamed artist

Pixiv's `artist` is the display name the artist chose and can change; a numeric user id
would survive a rename. The owner's answer (2026-09-23) is Danbooru's: when an artist
renames, search the old tag and bulk-retag to the new one. That answer makes a stable id
unnecessary as the tag, and a tag of digits is unreadable in every place tags are read. The
Pixiv adapter does not extract a user id today, and adding it would be an extension change
this change does not make. X's handle can be renamed too; the same answer covers it, and the
handle is what `account:` already matches.

**Amended 2026-09-25 (`artist-entries`):** the manual bulk retag above was right for the case
that existed then — a renamed artist corrected once, by hand, over images already stored — but
it left every *future* capture from the renamed handle wrong again, since nothing remembered
the correction. `artist-entries` keeps this paragraph's reasoning (the tag stays the readable
name; a stable id is still not the tag) and adds what was missing: the correction is now an app
action (`artists::rename`) that also records the source — the URL with the id — as an entry, so
the next capture matches it before falling back to the field this D4 derives. The Pixiv adapter
now does extract a user id (`site-adapters` `artist-entries` delta), specifically so that URL
survives a display-name change the way X's handle already did.

### D5. Nothing here can fail a capture that would otherwise succeed

`artist_tag` is total (every odd shape is `None`); `Skip` never errors; the only error left is
a database error from the insert/link statements, which already fails the transaction today
for every tag a capture carries. The `categorised` answer feeds the existing `library.json`
rewrite, whose failure already fails the call for rule-created categories; this change adds
no new failure kind there, only a new way to reach it.

### D6. The Inspector comment follows the spec

`Inspector.svelte`'s comment on the Account row (≈ line 708) repeats the old argument ("not
among the tags — with artist tags in the vocabulary a handle there read as a second
artist"). It is rewritten to the delta's: a fact of the page address, apart from the tags
because `account:` searches the address; the handle reaches the tags only as the derived
artist tag. No markup changes.

## Risks / Trade-offs

- [Pixiv display names often carry noise — `name@お仕事募集中`, `name｜C103 day2`] → the tag
  is that whole string, spelled. Accepted: the owner's rename answer (D4) is also the answer
  here. After a retag the old artist tag survives carrier-less (`tag-vocabulary`, "The
  vocabulary outlives its carriers") and the next capture from that artist links it again;
  aliases are a `tag-vocabulary` non-goal, so this stays until they exist.
- [A name the search reads as syntax — a leading `-`, or exactly `or`] → the tag is created
  but typing it into the search reads it as an exclusion or an or-group. The editor already
  accepts the same names, so this is not new; not handled here.
- [Case differs between the Account row and the tag] → the row shows the page address's
  segment as written (`@Alice`), the tag is lower-case (`alice`). Both find the image;
  the delta's scenario states it.
- [The derived tag and the rules both run in `insert_rows`'s transaction] → one more
  `SELECT category` and at most two inserts per capture, under the library mutex. Negligible
  beside the image decode already in that hold.

## Migration Plan

None: no schema change, no data patch. Images stored before the change keep their tags;
`arttags:0` (`category-count-search`) lists them. Rolling back is reverting the commit; artist
tags created meanwhile stay as ordinary artist tags.
