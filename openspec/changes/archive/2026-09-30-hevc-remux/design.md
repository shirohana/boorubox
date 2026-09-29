## Context

`media::probe` already walks to the first video sample entry's fourcc when `mp4parse` says
Unknown (`hevc-mp4` design D1); for H.264 it reports `avc1` whether the entry is `avc1` or
`avc3`. `ingest::store_image` writes the bytes through the inbox part file, then inserts the
row. The sidecar (`scripts/build-ffmpeg.sh`) has no muxer but `image2pipe`; a build adding
`--enable-muxer=mp4 --enable-bsf=hevc_mp4toannexb,extract_extradata` measured 3,201,312
bytes against 3,213,864 (2026-09-29) and remuxed the owner's clip to `hvc1`.

## Decisions

### D1 — Remux in-band entries at import, file to file, through the inbox

`media::probe` reports the entry's fourcc for every mp4 (the walk runs for named entries
too, so `avc3` is told from `avc1`). In `store_image`, after `write_through_inbox` has the
part file on disk and before the rename, an mp4 whose codec is `hev1` or `avc3` is remuxed by
`ffmpeg::remux(part, part2, tag)`: `-nostdin -loglevel error -i <part> -c copy -tag:v <hvc1|
avc1> -movflags +faststart -f mp4 <part2>`, then `part2` replaces `part`, and the file is
probed again from disk so the row records what was stored (`hvc1`, the new size). File to
file, not a pipe: a moov-at-end mp4 cannot be demuxed from a pipe, and the file is on disk
already. A remux error is an import failure carrying ffmpeg's last stderr line, never a
silently stored `hev1`: the file would import and then not play, which is the bug this change
fixes. The size stored is the remuxed file's.

### D2 — `codec` is a nullable column, in the sidecar, no sidecar version bump

Migration `ALTER TABLE images ADD COLUMN codec TEXT` — its number is `MIGRATIONS.len()` when
the unit lands (v16, when the unit landed). `ImageRecord.codec:
Option<String>` (`codec` in `packages/shared`), `Sidecar.codec` optional, `recover` restores
it. Values are the container's own codes as `Probed.codec` already spells them: `avc1`,
`hvc1`, `vp08`, `vp09`, and for webm the Matroska id mapped to the same four-letter form
(`V_VP8` → `vp08`, `V_VP9` → `vp09`, `V_MPEG4/ISO/AVC` → `avc1`, `V_MPEGH/ISO/HEVC` →
`hvc1`). Existing rows stay null; the viewer treats null as "try to play" (today's behaviour).

### D3 — The viewer asks before it plays

`Lightbox.svelte`, for a video with a codec: `canPlayType` on a detached `<video>` with the
MIME and a codecs string per codec (`hvc1` → `hvc1.1.6.L93.B0`, `avc1` → `avc1.42E01E`,
`vp08` → `vp8`, `vp09` → `vp09.00.10.08`), in a pure helper `canDecode(mime, codec)` in a
new `lib/domain/codecs.ts` with tests. An empty answer means the engine will not decode it:
the view shows the message in the picture's place without mounting the `<video>` (audio
alone must not start). The message names the codec ("This machine's browser engine cannot
decode HEVC (H.265)") and, on Windows (`isWindows` from `$lib/platform`, or the same rule the
platform module already exposes), adds one sentence: "Windows' engine plays HEVC only with
Microsoft's HEVC Video Extensions." No link, no purchase prompt. The existing `error`-event
message stays for the cases the engine promises and then fails.

### D4 — The sidecar build

`--enable-muxer=image2pipe,mp4 --enable-bsf=hevc_mp4toannexb,extract_extradata` added to
the configure line; the script header explains why (the remux of in-band parameter sets into
`hvcC`/`avcC`). The stamp changes, so `mise run ffmpeg` and both CI caches rebuild.

## Risks

- A clip whose `hev1` stream lacks in-band parameter sets at the start cannot become a valid
  `hvc1`; ffmpeg errors, the import fails by name. Rare; the owner raises it if met.
- `canPlayType` answers "maybe" for codecs the engine is unsure of; only the empty string
  blocks playback.
