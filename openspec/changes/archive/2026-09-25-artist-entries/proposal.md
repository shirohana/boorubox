## Why

Since `auto-artist-tag` (2026-09-24) every X and Pixiv capture carries an artist tag derived
from the handle or the display name. Sometimes that is not the artist's tag: `metaljelly0811`
on X is `metaljelly` on Danbooru, and a Pixiv display name can read `name@お仕事募集中`. The
owner corrects an artist's name only once they have saved a lot of that artist's work, and
wants the correction to stick for future captures without maintaining a table of every artist
(owner, 2026-09-25). Danbooru's answer is the artist entry: a tag that owns a list of profile
URLs, matched against a post's source when it is created. Requirements §6 (Danbooru-style
tagging), §11 (policy lives in the app; the extension only extracts).

## What Changes

- **Artist entries**: an artist tag owns a list of profile URLs, stored with the library and
  restored on rebuild. Only corrected artists have an entry.
- **Derivation by URL first**: at capture the record's profile URL (X from the handle, Pixiv
  from the artwork's user id), the post URL and the page URL are matched against the entries'
  URLs as prefixes; a match names the artist tag, otherwise today's handle or display name does.
  Every X and Pixiv capture still gets one artist tag, under the same category rule as today.
- **Rename an artist where the wrong tag is seen**: an artist tag in the inspector offers
  "Rename artist…"; the dialog shows the new name, the URLs read from that image, and how many
  images carry the old name; confirming writes the entry and retags every carrier to the new
  name, merging into it if it already exists.
- **Settings → Artists**: the entries, one per artist with its URLs, add, edit URLs, delete;
  with the notice that URL edits apply to future captures only.
- **The Pixiv adapter extracts the user id** from the artist anchor it already reads.

## Capabilities

### New Capabilities

- `artist-entries`: the entries, their matching, the rename action, the settings section.

### Modified Capabilities

- `capture-ingest`: the artist tag is the matching entry's tag before it is the handle or
  display name.
- `site-adapters`: the Pixiv record carries `userId`.

## Non-goals

- A general "rename a tag" for any category: bulk edit is the door for that (owner,
  2026-09-25). Only an artist tag renames, because an artist's name is a fact about a source,
  not about the picture.
- Re-deriving artist tags over images already stored when an entry is edited in Settings. A
  FIXME names the pass; the rename dialog's retag covers the case that exists today.
- Other names (Danbooru's `other_names`), aliases for non-artist tags, implications.
- Matching by display name or handle text: URLs with ids survive renames, names do not
  (owner, 2026-09-25).
- Extracting Pixiv's user id for captures already stored: they are corrected by the rename
  dialog's retag, by name, once.
