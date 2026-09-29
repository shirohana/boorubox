## Context

Every door stores a file through `ingest::store_image`, which sniffs and fully decodes the
bytes with the `image` crate (`ingest::decode`) to get `ext`, `mime`, `width` and `height`,
all NOT NULL on `images`. Thumbnails (`thumbs.rs`) re-decode the source with the same crate,
downscale to `THUMB_EDGE` and encode a JPEG through a part file. The webview loads the file by
`convertFileSrc` over Tauri's asset protocol, which answers Range requests with 206 (tauri
2.11.5, checked), so a `<video>` element streams and seeks without any Rust work. Rust has no
video decoder and the app bundles no native tools. The `is:` metatag is parsed in
`tag-utils.ts` into a MIME list that `query.rs` turns into `images.mime IN (...)`.

Spike, 2026-09-29 (M2 Max): the standard static ffmpeg is 81 MB and Intel-only; ffmpeg 7.1.1
configured with `--disable-everything`, the mov and matroska demuxers, the h264/hevc/vp8/vp9
decoders, the mjpeg encoder and image2 muxer, `--enable-small`, no GPL parts, no network, is
3.2 MB stripped on arm64 and extracted a poster from a libx264 mp4 and a VP9 webm. The
alternatives were weighed and rejected: the openh264 crate (0.6 MB) is H.264 only and needs
its own mp4 demux; a webview canvas capture is free but overrides §6 ("the webview owns UI
only") and cannot run under the gate (jsdom has no decoder); a placeholder tile gives the grid
nothing to tell one clip from another.

## Goals / Non-Goals

Goals: mp4 and webm in through every door; facts from the header; a first-frame thumbnail
rendered by a bundled extractor on both platforms; looping muted playback in the viewer; type
search; the duration where the facts are.

Non-goals: see the proposal. Additionally not decided here: a frame other than the first, a
timeout on the extractor (FIXME), streaming reads (FIXME).

## Decisions

### D1 — Acceptance by container and codec, in Rust, from the bytes

A new `media` module owns "what is this file": `media::probe(bytes) -> Result<Probed>` where
`Probed { kind: Image | Video, ext, mime, width, height, duration_ms: Option<i64>,
codec: Option<&'static str> }`. It tries the `image` crate's `guess_format` first (the still
path is unchanged: a full decode, dimensions from the pixels); when that says unknown it looks
for an MP4 (`ftyp` at byte 4) or an EBML header (`1A 45 DF A3`) and parses the header with a
pure-Rust crate — the `mp4` crate for MP4 (`Mp4Reader::read_header` over a `Cursor`, first
video track: `width()`, `height()`, `duration()`, `media_type()`/sample entry) and the
`matroska` crate for WebM (DocType must be `webm`; first video track's `PixelWidth`,
`PixelHeight`, segment `Duration`, `CodecID`). Accepted codecs: MP4 sample entries
`avc1`/`avc3` (H.264) and `hvc1`/`hev1` (HEVC); Matroska `V_VP8` and `V_VP9`, plus
`V_MPEG4/ISO/AVC` and `V_MPEGH/ISO/HEVC` for an mkv-flavoured webm. Anything else, or no
video track, is `AppError::Unsupported(reason)` — a new variant beside `Decode`, so
`import_file` can map it to *skipped* with the reason (`"video codec mpeg4 is not supported"`,
`"no video track"`) while `Decode` keeps meaning "not a file we know". `ext` is `mp4` /
`webm`, `mime` is `video/mp4` / `video/webm`; a QuickTime brand (`qt  `) is still `mp4`
here: the webview plays it, and Danbooru names the container by the same extension.

**Amended at apply time (U2, 2026-09-29): the MP4 half accepts H.264 only; HEVC in an mp4 is
refused as `unknown`.** The `mp4` crate was the plan's parser; it reads `hvc1`/`hev1` but
silently drops every sample entry it does not know (`mp4v`, `av01`, …), so a refusal could not
name its codec, and naming the codec is the requirement that lets the owner tell a refused file
from a corrupt one. `mp4parse` (Mozilla's) names every entry it knows and reports the rest as
`Unknown` — but its table has no HEVC. Naming refusals won over accepting HEVC because the
owner's material is H.264 and they asked to raise unsupported formats as they meet them. The
right shape when that happens: a targeted read of the first `stsd` entry's fourcc when
`mp4parse` says `Unknown` (a FIXME in `media.rs` names it). WebM/Matroska is as planned: the
`matroska` crate names the codec id, so `V_MPEGH/ISO/HEVC` is accepted there. The MP4
duration is the video track's own (`mp4parse` does not expose the movie header's); the
DocType check for WebM is a search for `webm` in the first 64 bytes, since the crate skips the
EBML header.

Why refuse by codec at the door: the bundled extractor and the two webviews decode exactly
these four; a file outside them would be stored as a tile with no preview that will not play,
which is indistinguishable from a bug. The refusal names the codec so the owner can raise it
(their call, 2026-09-29: "if I hit an unsupported format I'll raise it"). Why header parse
and not the sidecar's own `-i` output: dimensions must be available with the library lock
held and without a process spawn per file, and the parsers are pure Rust, no build cost.

### D2 — The poster comes from a bundled minimal ffmpeg, spawned by Rust

`scripts/build-ffmpeg.sh` downloads ffmpeg 7.1.1 from ffmpeg.org (sha256 pinned in the
script), configures it with exactly the spike's line (`--disable-everything --disable-doc
--disable-ffprobe --disable-ffplay --disable-network --disable-autodetect --disable-avdevice
--disable-postproc --disable-debug --disable-x86asm --enable-small --enable-protocol=file,pipe
--enable-demuxer=mov,matroska --enable-decoder=h264,hevc,vp8,vp9
--enable-parser=h264,hevc,vp8,vp9 --enable-encoder=png --enable-muxer=image2pipe
--enable-filter=scale,select,format`), builds, strips, and writes
`packages/app/src-tauri/binaries/ffmpeg-<target triple>[.exe]` plus a stamp file holding the
script's own sha256, and skips the whole build when the stamp matches. No `--enable-gpl`: the
result is LGPL 2.1, shipped as its own process, with the licence text beside the binary in the
bundle (`bundle.resources`) and the configure line in the script — that is the LGPL's
"tell them how it was built". `--disable-x86asm` so the Windows build needs no nasm: one
keyframe per file is not where a SIMD decoder pays. `pipe` protocol and `png` encoder rather
than `mjpeg` + `image2`: ffmpeg writes the raw first frame as one PNG to stdout and Rust's
existing downscale-and-JPEG path finishes it — one definition of the thumbnail's size, quality
and part-file write for both kinds (ZERO DUPLICATION); this also retires the FIXME shape in
`thumbs.rs` partway, since `write_through_part` now takes a `DynamicImage` from either
decoder.

`tauri.conf.json` gains `bundle.externalBin: ["binaries/ffmpeg"]`; Tauri names the file
`ffmpeg` (`.exe` on Windows) beside the app executable, in the bundle and in `target/debug`
under `tauri dev`. tauri-build refuses to build when the file for the target triple is
missing, so the script is a prerequisite of `cargo build` and `cargo test`: a `mise run
ffmpeg` task runs it, `mise run check` depends on it, and the two workflows run it before
tauri-action. `binaries/` is gitignored.

The portable `boorubox.exe` release asset has no `ffmpeg.exe` beside it, and `ffmpeg::locate`
looks only next to the executable, so the portable copy shows no video thumbnails (videos
still import and play). Not fixed: the portable exe is one file by definition; shipping a
second file beside it is what the installer is for. The release body says so.

A new `ffmpeg` module locates and runs it. In the app: `current_exe().parent()/ffmpeg[.exe]`.
Under `cargo test` the test binary lives in `target/debug/deps`, so `#[cfg(test)]` resolves
`env!("CARGO_MANIFEST_DIR")/binaries/ffmpeg-<triple>` instead, with the triple handed to
`env!` by `build.rs` (`cargo:rustc-env=BOORUBOX_TARGET=$TARGET`). Two explicit cases, no
search list. `ffmpeg::first_frame(path) -> Result<DynamicImage>` spawns
`ffmpeg -nostdin -loglevel error -i <path> -frames:v 1 -f image2pipe -vcodec png -`, uses
`std::process::Command` (the shell plugin is for the webview; Rust needs none of it), sets
`CREATE_NO_WINDOW` on Windows so no console flashes, reads stdout to the end, and decodes the
PNG with the `image` crate. A non-zero exit is an error carrying the last line of stderr.
FIXME in the module: no timeout; a hung extractor blocks the caller's thread, not the library
lock (thumbnails are already generated with the lock released, `thumbs.rs` design D13).

Thumbnails: `write_through_part` grows a first step that picks the decoder by the record's
MIME (`video/*` → `ffmpeg::first_frame`, else the `image` crate) and is otherwise unchanged;
`ensure_thumbnail`, `warm_thumbnail` and `regenerate_all` need no change beyond passing the
MIME. A poster failure behaves as an image thumbnail failure does today: swallowed after
import, counted as failed by the regenerate pass, "No preview" on the tile.

Windows build: `windows-test-build.yml` and `release.yml` add an MSYS2 step
(`msys2/setup-msys2`, `msystem: MINGW64`, packages `mingw-w64-x86_64-gcc make diffutils`)
that runs the same script under `shell: msys2 {0}` with `--extra-ldflags=-static` so the
result depends on no MinGW DLL; the script renames `ffmpeg.exe` to the MSVC triple name
tauri-build expects (`ffmpeg-x86_64-pc-windows-msvc.exe`) — the name is Tauri's lookup key, not
a statement about the compiler. macOS runs the script under bash with the runner's clang.
Both cache `binaries/` with `actions/cache` keyed on the runner OS and the script's hash, so a
release does not rebuild ffmpeg unless the script changed.

### D3 — `duration_ms` is a nullable column, in the sidecar, no sidecar version bump

Migration `ALTER TABLE images ADD COLUMN duration_ms INTEGER` — its number is
`MIGRATIONS.len()` when the unit landed: v15. `ImageRecord.duration_ms: Option<i64>` (`durationMs` in `packages/shared`),
`Sidecar.duration_ms` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, and
`recover` copies it back into the row. `SIDECAR_VERSION` stays 1: the version is bumped only
when a v1 reader would get the shape wrong, and a v1 reader ignoring an optional field it does
not know gets nothing wrong. Images carry no duration (the field is absent from their sidecar).

### D4 — `is:video`, `is:mp4`, `is:webm` are parser-only

`tag-utils.ts` extends the `is:` regex and the value-to-MIME table: `video` adds both
`video/mp4` and `video/webm`, `mp4` and `webm` one each. `query.rs`'s `mime IN (...)` is
already the right shape. `stamp.ts`'s search-only list already names `is:`.

### D5 — The viewer plays a video with the platform's controls and no zoom

`Lightbox.svelte` renders `<video>` instead of `<img>` when `image.mime` starts with
`video/`: `autoplay loop muted playsinline controls`, `draggable="false"`, same `src`. The fit
uses the same math: `loadedmetadata` sets `naturalSize` from `videoWidth`/`videoHeight`, so
`fitScale`, `content` and the style are shared. For a video `toggleZoom`, the wheel handler
and the pinch handlers return early (`isVideo` derived from the record) — the click is the
controls', and the spec's zoom clauses are image clauses. Space and Escape close as they do
now; the surface keeps focus (the `<video>` has `tabindex="-1"` so Tab does not stop on it —
the controls are pointer targets, the spec says Tab reaches only the view's own controls).
`move()` already resets `naturalSize`; the new element mounts fresh for the next item, so
playback starts at the beginning. On the element's `error` event the view shows a short
message in the picture's place ("This video cannot be played on this machine"); the keys are
on the surface and keep working. `bind:this` is not needed; `loop` restarts the element
itself.

Why muted: both webviews may refuse unmuted autoplay without a gesture, and the owner asked
for muted. Why native controls rather than the app's own: sound and seeking need a control
surface and the spec forbids drawing the app's own chrome over the picture; the platform's
controls are the platform's, not ours.

### D6 — The tile and the Size row

`ImageCard.svelte`: when the record has a `durationMs`, a small badge with `formatDuration`
(m:ss) sits in the tile's bottom-left corner, opposite the existing badge cluster, over the
thumbnail or the no-preview state alike. `format.ts` gains `formatDuration(ms)` and
`formatSizeLine` takes an optional `durationMs`, appending `, m:ss` inside the parentheses.
The inspector passes the record through as it does today.

### D7 — Fixtures are checked in, generated once

`packages/app/src-tauri/fixtures/video/`: `h264.mp4`, `vp9.webm`, `mpeg4.mp4` (refused codec),
`audio-only.mp4` (no video track), `png-named.mp4` (a PNG renamed), each a few KB, 16×16,
two frames, generated with the owner's ffmpeg by the lead and committed. CI never needs
ffmpeg to run the tests; it needs the sidecar, which the script builds.

## Risks / Trade-offs

- **The MSYS2 build of ffmpeg is the one moving part nobody has run yet.** Mitigation: it is
  its own unit, proven by dispatching `windows-test-build.yml` before the Rust unit starts;
  the mac half is proven locally. Fallback if it fights back: the webview capture (§6
  override, owner's call) or the placeholder tile.
- **A local `cargo test` now needs `mise run ffmpeg` once** (about two minutes, then cached
  by the stamp). Recorded in CLAUDE.md's Commands.
- **Whole-file reads** stay: a 500 MB clip is a 500 MB allocation on import; FIXME on
  `import_file` naming the streaming shape.
- **Rotation**: an mp4 with a rotation matrix reports its stored, not displayed, dimensions;
  the viewer's fit uses `videoWidth`/`videoHeight`, which the webview reports rotated, so the
  picture is right and the Size row may be swapped. Non-goal, noted in `media`.

## Migration Plan

One additive migration; existing sidecars read unchanged; a rebuild fills `duration_ms` from
sidecars that carry it. The sidecar binary is new in the bundle: an install taking this update
through the updater gets it with the app (it is inside the `.app` / installed by NSIS).

## Open Questions

None for the owner. For the lead at apply time: the real migration number.
