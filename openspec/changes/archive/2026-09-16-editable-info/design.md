## Context

See proposal.md — Why. Single-image writes follow one shape (`tags::update_tags`,
`tags::set_rating`): a transaction, `stamp(&tx, id, …)` first so a missing image refuses the
edit before anything is written, commit, then `sidecar::write_one` after the commit
(`library-sidecars` D4), then `ingest::require_record` to answer with the changed row. The FTS
index is maintained by triggers on `images` (`page_title`, `page_url`, `image_url`), so an
`UPDATE` re-indexes by itself. The webview's `SearchResults.saveTags` / `saveRating` call the
command and `replace()` the row in place, never re-running the search (browse D10).
`Inspector.svelte` draws the facts as a `<dl>`; `inspector-polish` adds `onrelease` and the
`account` field derived from `page_url`.

## Goals / Non-Goals

**Goals:** one write for the three facts, in the existing write shape; the form is the panel's
rows, not a second surface.

**Non-Goals:** per-field saves; editing in the grid or the tile menu.

## Decisions

### D1. One write, `facts::update(library, id, &FactsEdit) -> ImageRecord`

`FactsEdit { page_title, page_url, image_url }`, each `Option<String>`: `None` or an empty
string after trimming stores `NULL`. Addresses are validated before the transaction: a
non-empty one must start with `http://` or `https://` after trimming and parse as a URL
(`url` crate if already a dependency; else the scheme check alone — grep `Cargo.toml`), or
`AppError::BadRequest("… is not a web address")`. Then the transaction: `UPDATE images SET
page_title, page_url, image_url, updated_at`, refusing a missing id the way `stamp` does (reuse
`stamp` for the timestamp, then the three-column update, or one statement — the agent picks
the one that keeps the "missing image refuses first" rule). Commit, `sidecar::write_one`,
`require_record`. Module `facts.rs`, named by the spec's word for these rows.

### D2. The command and the store

`update_facts(id, edit: FactsEdit) -> ImageRecord`; wrapper `updateFacts(id, edit)`;
`SearchResults.saveFacts(id, edit)` → `replace(record)`. The replaced record carries the new
`account`, so the account entry follows without a second call.

### D3. The form replaces the three rows, in the `<dl>`

`editing` is component state. The Edit action is a small ghost icon button in the panel's
header row (pencil), shown only when a single image is described. While editing, the Title,
Page and Image `<dd>`s become `Input`s (the same `text-xs` scale as the rows), and a row of
`Save` / `Cancel` follows the three; the open-link buttons are hidden while editing (there is
nothing to open until saved). Enter in any field saves, Escape cancels — with `preventDefault`
and `stopPropagation`, because inside the viewer a native `<dialog>` would otherwise close on
the same Escape. A save that fails shows the reason under the row and keeps the draft; a save
that succeeds leaves editing mode and calls `onrelease`. Changing the described image while
editing discards the draft (the image the draft was for is gone from the panel).

*Alternative rejected:* a dialog. The rows are the fields; a dialog would show them a second
time a few hundred pixels away.

### D4. Validation lives on the Rust side only

The webview does not pre-validate the addresses: the rule is one function, the refusal is the
message, and a second copy in TypeScript would drift (CLAUDE.md, policy lives in the app —
here, the Rust half of it).

## Risks / Trade-offs

- [A page address edited to an X post changes grouping and the account filter] → intended;
  the spec scenario says so.
- [The title edit changes the viewer's and the tile's title] → intended; both derive from the
  record.
