## Context

`media::probe_mp4` reads the header with `mp4parse`, which reports an `hvc1`/`hev1` sample
entry as `SampleEntry::Unknown` carrying nothing — no fourcc, no dimensions. `video-files`
design D1 (amended) chose that parser because it names every codec it knows and the
alternative drops unknown entries silently; the FIXME in `media.rs` names this change's shape.

## Decisions

### D1 — A targeted fourcc read when the parser says Unknown

A small box walk, `first_video_sample_entry(bytes) -> Option<[u8; 4]>`: `moov` → each `trak`
→ `mdia` → `hdlr` (take the first trak whose handler is `vide`) → `minf` → `stbl` → `stsd` →
the first entry's type. Box headers are 8 bytes (size, type), `largesize` (size 1) is 16,
size 0 means "to the end"; `stsd` is a full box (4 bytes version/flags, 4 bytes entry count)
before its entries. Every read is bounds-checked and returns `None` on a malformed box; it
never allocates. Used only on the `Unknown` path, so the H.264 path is unchanged and
`mp4parse` stays the parser of record (this is the "targeted read" the FIXME named, not a
second parser: it answers one question, "what is the first video sample entry's type").

Accepted from that read: `hvc1` and `hev1`, both reported as their own fourcc in
`Probed.codec`. Any other fourcc is `video codec <fourcc> is not supported`, printed as the
four ASCII bytes (non-ASCII bytes as `?`). A file where the walk finds nothing keeps today's
`video codec unknown is not supported`.

### D2 — Dimensions from the track header

For an `Unknown` entry the dimensions come from `track.tkhd` (`mp4parse` exposes it): width
and height are 16.16 fixed point, so `>> 16`; then `checked_dimensions` as for every video.
The H.264 path keeps its sample-entry dimensions.

### D3 — Fixtures

`fixtures/video/hvc1.mp4` and `hev1.mp4`: 16×16, two frames, libx265, generated once by the
lead with the local ffmpeg; the shipped sidecar renders a poster from both (checked).

## Risks

- WebKit plays HEVC tagged `hvc1`; `hev1` files may refuse to play there. WebView2 needs
  Microsoft's HEVC Video Extensions. Import accepts both tags because the sidecar renders
  both and the viewer already shows a plain message when the machine cannot decode a file.
