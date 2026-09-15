## Why

The owner's Windows pass (2026-09-15): "The image information should be editable. But it's not
a frequent action, don't replace the fields with text input directly, add an edit button." The
case is local imports (§6 Sources: "metadata = filename, mtime, dimensions"): an image dropped
in from a folder has no title, no page address and no image address, and today nothing can
ever give it one. The inspector shows those three as read-only text (`app-frame`, "Read-only
stands in for an editor" — a placeholder rule from the first build, now overtaken for these
three fields).

**Depends on:** `app-shell` (the inspector), `library-sidecars` (every write to an image writes
its sidecar), archived; lands after `inspector-polish` (this delta carries its text for the
shared requirement, and uses its `onrelease` hand-back).

## What Changes

- **Title, page address and image address can be edited** from the inspector, in both
  placements, behind an Edit action: pressing it turns those three rows into a small form with
  Save and Cancel; nothing else in the panel changes shape. Saving writes all three, stamps the
  image as changed, writes its sidecar, and redraws the image everywhere it is shown. An
  address that is not empty MUST be an `http` or `https` address; otherwise the save is
  refused with a reason and the form stays open.
- **Everything the edit feeds follows**: the free-text search index, the `account:` filter and
  the X-account grouping all read the page address, so an image given an X post's address is
  found by them afterwards.
- The other facts — dimensions, size, type, times, id — stay read-only: they are measured, not
  entered.

## Non-goals

- Editing capture time, source or the file itself.
- Bulk editing facts across a selection.
- A dialog: the panel is already the place these facts are read, so it is where they are
  written.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-frame`: "One inspector panel, two placements" — three facts become editable behind an
  edit action.

## Impact

- `packages/app/src-tauri`: a new `facts` module with `update` (validate, write, stamp,
  sidecar), command `update_facts`; `packages/shared` gains `FactsEdit`; `api/commands.ts`
  wrapper; `SearchResults.saveFacts`; `Inspector.svelte` gains the Edit action and the form.
  Schema and sidecar format unchanged: the sidecar already carries all three fields.
