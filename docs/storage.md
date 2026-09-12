# How BooruBox stores your library

A library is one folder you choose. Everything BooruBox knows about your images lives inside
it, in files you can open with other tools. Copying the folder is a complete backup; moving
it to another machine and opening it there is a complete move. This page says what is in the
folder, why each part is there, and what that means if the folder lives in a cloud-synced
location.

## What is in the folder

```
<library>/
  images/<a1>/<b2>/<id>.<ext>   the image itself, untouched
  images/<a1>/<b2>/<id>.json    everything BooruBox knows about that image
  library.json                  the auto-tag rules, the booru sites and the library note
  library.sqlite                the search index, built from the two kinds of file above
  .thumbs/                      thumbnails, regenerated whenever they are missing
  inbox/                        files being written; empty when the app is idle
```

**The images are plain files.** BooruBox never rewrites or re-encodes an image after saving
it. The two-level folders under `images/` are named from the first four characters of the
image's id, so no single folder ever holds more than a small share of the library: Finder,
Explorer and sync clients all slow down when one folder holds tens of thousands of files.

**Beside every image sits a small JSON file** with the same name. It holds the tags, the
rating, where the image came from, when it was saved, whether it is in the trash, and which
boorus it has been posted to. BooruBox writes it in the same moment it saves any of those
facts, so the file is never behind. It is written for people and for recovery, not as an
input: editing it by hand changes nothing in the app, and a rebuild would overwrite the edit
with what the app knows.

**`library.json`** does the same job for the handful of things that are not about one image.
Booru API keys are never in it; those stay in the operating system's credential store.

**`library.sqlite`** is what makes search instant. It is an index over the JSON files, and
that is the point of this layout: the index can be thrown away and built again from the
folder, and nothing is lost.

## Rebuilding the index

If the app tells you a library is damaged, its database failed a check when it was opened.
The images and their JSON files are still there. Choose *Rebuild*, and BooruBox:

1. Builds a fresh database from every `<id>.json` and from `library.json`, without opening
   a single image.
2. Moves the damaged database aside as `library.sqlite.corrupt-<number>` (and its journal,
   if one exists) and puts the new one in its place.
3. Opens the library and reports how many images came back and which files, if any, it
   could not read.

The same command is in Settings, under the library, for a database that opens but looks
wrong. BooruBox never deletes the file it moved aside. Once the rebuilt library looks right
to you, delete it yourself.

A rebuild recovers everything the JSON files describe. What it cannot recover is an image
that has no JSON file beside it: rather than invent tags for it, the rebuild leaves it
alone and does not list it. The first time this version opens an older library it writes
the missing JSON files in the background; a small tile shows the progress, and the library
is usable throughout.

## Cloud-synced folders

Putting your library in a folder that Dropbox, iCloud Drive, OneDrive, Google Drive or
Synology Drive syncs is fine, with one rule: **only one machine writes to it at a time**.
Open it on your laptop, close the app, let the sync finish, then open it on your desktop.

The reason is the database. A sync client copies `library.sqlite` while BooruBox is writing
it, and a bidirectional client can copy a half-written version back. SQLite then reports
the file as malformed and the app refuses to open it. This is not a bug in the client or in
SQLite; it is what happens to any database file that two programs write. It happened to a
BooruBox library during a large import in September 2026, and it is why the JSON files
exist: when it happens, the index is rebuilt from them and the library is whole again.

What the JSON files do not do is merge. If two machines edit the same library at the same
time, the sync client will keep two versions of some files, and neither BooruBox nor a
rebuild will reconcile them. One machine at a time keeps everything simple.

Two more things to know:

- A sync client sees two files per image, not one, and a burst of small writes during an
  import. That is normal.
- A synced folder is convenient, but it is not a backup against your own mistakes: the
  client faithfully syncs a deletion too. A copy of the folder somewhere the client does
  not touch is the backup.
