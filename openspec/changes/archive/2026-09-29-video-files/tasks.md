> Four units. U0 is the lead's own checkpoint; U1 and U2 are serial (U2 cannot build without
> U1's binary); U3 runs beside U2 (disjoint files once U0 has landed). Design D1–D7 decide
> every shape; do not re-decide them. The migration number is whatever `MIGRATIONS.len()` is
> when U2 lands (14 today, planned v15): amend D3's sentence to the real number, never pin the
> planned one. Gate for every unit: `mise run check` (which now runs `mise run ffmpeg` first).
> No unit commits or ticks a hand check; hand checks get a `Hand check:` line.

## 0. Unit U0 — shared type and fixtures (lead)

- [x] 0.1 `packages/shared/src/index.ts`: `durationMs?: number | null` on `ImageRecord`, doc
      comment: milliseconds, videos only, absent on an image.
- [x] 0.2 `packages/app/src-tauri/fixtures/video/` per D7, generated with the local ffmpeg;
      `.gitignore` gains `packages/app/src-tauri/binaries/`.
- [x] 0.3 Gate green (nothing reads the field yet). Commit.

## 1. Unit U1 — the sidecar build (`scripts/`, `mise.toml`, `packages/app/src-tauri`, `.github`) — Sonnet

- [x] 1.1 `scripts/build-ffmpeg.sh` per D2: pinned version and sha256, the configure line,
      the stamp-and-skip rule, the Windows rename and `-static` flags under `MSYSTEM`, strip,
      output path by the target triple (`rustc -vV` host, or `$1` to override). Comment at the
      top says why the configure line is what it is (the four codecs the webviews decode; no
      GPL; no nasm) and how to add a decoder.
- [x] 1.2 `mise.toml`: `tasks.ffmpeg` runs the script; `tasks.check` runs it first. CLAUDE.md
      Commands: one sentence — `cargo build`/`cargo test` need the sidecar, `mise run ffmpeg`
      builds it once.
- [x] 1.3 `tauri.conf.json`: `bundle.externalBin: ["binaries/ffmpeg"]`; `bundle.resources`
      carries the LGPL text (`packages/app/src-tauri/binaries/LICENSE.ffmpeg`, written by
      the script from the tarball's `COPYING.LGPLv2.1`). `build.rs`: `BOORUBOX_TARGET` per D2.
- [x] 1.4 `.github/workflows/release.yml` and `windows-test-build.yml`: the sidecar step before
      tauri-action on both platforms per D2, with `actions/cache` on `binaries/` keyed by
      runner OS and the script's hash. Comment on the step says why MSYS2 (the script is a
      configure-and-make build; PowerShell cannot run it) and why the MSVC triple name.
- [x] 1.5 Gate: `mise run ffmpeg` produces `binaries/ffmpeg-aarch64-apple-darwin` and a second
      run skips; `pnpm --filter @boorubox/app tauri build --no-bundle` succeeds with the
      externalBin wired; `./binaries/ffmpeg-… -i fixtures/video/h264.mp4 -frames:v 1 -f
      image2pipe -vcodec png - | wc -c` is non-zero. Report the binary's size.
- [x] 1.6 (lead) Dispatch `windows-test-build.yml` on the committed branch; the run is green
      and the artifact's installer holds `ffmpeg.exe` beside `boorubox.exe`. Not a hand check:
      the run log is the evidence.
      Evidence: branch `test-build/video-sidecar`, run 36572747807, success; the sidecar step
      logged `built …/binaries/ffmpeg-x86_64-pc-windows-msvc.exe` on windows-latest.

## 2. Unit U2 — probe, ingest, poster, duration (`packages/app/src-tauri`) — Sonnet

- [x] 2.1 `media.rs` per D1 (`probe`, `Probed`, the accepted-codec table, `Unsupported`
      reasons), `error.rs` gains `Unsupported(String)`. `Cargo.toml`: `mp4` and `matroska`
      with a comment saying why header parsers and not the sidecar. Tests on the fixtures:
      `h264_mp4_is_probed_as_video` (1664 is not the fixture — assert the fixture's own 16×16
      and its duration), `vp9_webm_is_probed_as_video`, `mpeg4_mp4_is_refused_by_codec_name`,
      `audio_only_mp4_is_refused_for_no_video_track`, `a_png_named_mp4_is_a_png`,
      `an_image_has_no_duration`.
- [x] 2.2 `ingest.rs`: `decode` becomes `media::probe`; `Decoded` carries `duration_ms`;
      `insert_rows` writes it. `import.rs`: `Unsupported` → skipped with the reason; the
      report scenario in `local-file-import` ("A refused video is named with its reason")
      as a test.
- [x] 2.3 `db.rs`: the migration per D3, appended to `MIGRATIONS`; test in the style of the
      existing migration tests. `model.rs`, `sidecar.rs`, `recover.rs`: `duration_ms` per
      D3; a recover test that a sidecar carrying `durationMs` restores the column and one
      without it restores null.
- [x] 2.4 `ffmpeg.rs` per D2 (`locate`, `first_frame`, the `cfg(test)` path, the FIXME);
      `thumbs.rs`: `write_through_part` picks the decoder by MIME. Tests: `first_frame_of_the_
      h264_fixture_is_16_by_16`, `a_video_thumbnail_is_a_jpeg_at_the_source_size` (16 px, no
      upscale), `regenerate_all_counts_a_video_it_cannot_render_as_failed` (point the record
      at a missing file).
- [x] 2.5 Gate green. Handoff below: the real migration number, the `Probed` shape, the
      `Unsupported` reason strings the report shows, the sidecar binary's location rule.
- [ ] 2.6 Hand check (owner): drop the `novel_script` folder on the window; every clip
      imports, each tile shows its first frame; Settings → Library → Regenerate thumbnails
      re-renders them.
      Seen (lead's smoke on a scratch copy of test-1, 2026-09-29, macOS): Import folder… on
      the 28 clips → "Imported 28 · skipped 0 · failed 0"; every tile showed its first frame
      with a `0:08` badge; `library.sqlite` held 28 `video/mp4` rows with durations 4067–8100
      ms. Regenerate thumbnails was not run.

## 3. Unit U3 — viewer, tile, search, Size row (`packages/app/src`) — Sonnet, beside U2

- [x] 3.1 `tag-utils.ts` per D4 with tests beside the existing `is:` tests: `is:video` yields
      both MIMEs, `is:mp4` one, `is:webm is:png` two.
- [x] 3.2 `format.ts`: `formatDuration` (`0:08`, `1:05`, `12:00`) and `formatSizeLine` with
      `durationMs` per D6, tests including the spec's `1.8 MB .mp4 (1664×1024, 0:08)` and an
      image's line unchanged.
- [x] 3.3 `ImageCard.svelte`: the duration badge per D6, over the thumbnail and over the
      no-preview state; a component test (jsdom, the `ImageCard`/`Inspector` test harness
      style) that a record with `durationMs` renders the badge and one without does not.
- [x] 3.4 `Lightbox.svelte` per D5: the `<video>` branch, `loadedmetadata` → `naturalSize`,
      zoom handlers inert for a video, the error message, `tabindex="-1"`. Component test:
      a video record renders a `<video>` with `loop`, `muted`, `autoplay`, `controls` and the
      record's URL; a click on it does not change the scale; an image record still renders
      `<img>`.
- [x] 3.5 `Inspector.svelte`: the Size row passes `durationMs` (one line). Gate green.
      Handoff below.
- [ ] 3.6 Hand check (owner, Mac and Windows): open a clip — it plays from the start, loops,
      is muted, the controls unmute and seek; wheel and pinch do nothing; Space closes;
      arrows move to the next clip, which starts from its beginning; `is:video` lists the
      clips; the Size row shows `(1664×1024, 0:08)`.
      Seen (same smoke): the viewer opened a clip playing with the platform controls and the
      muted icon, two captures a second apart differed, → moved to the next clip, `i` showed
      `1.1 MB .mp4 (1136×856, 0:08)`, Space closed the view; `is:video` listed 28. Not seen:
      unmute and seek through the controls, wheel and pinch, the failed-clip message, Windows.

## Handoff

(Each unit appends here: what landed, deviations, what the next unit needs.)

### U3

Landed (gate green: lint, typecheck, vitest 866): `tag-utils.ts` (`is:video|mp4|webm`),
`format.ts` (`formatDuration`, `formatSizeLine` takes `durationMs`), `ImageCard.svelte`
(bottom-left duration badge, outside the rating cluster's `{#if}`, so it shows over the
thumbnail and the no-preview state), `Lightbox.svelte` (`isVideo`, keyed `<video>`, error
message, zoom/wheel/pinch inert), new tests `ImageCard.svelte.test.ts` and
`Lightbox.svelte.test.ts`, plus cases in `tag-parser.test.ts` and `format.test.ts`.
Deviation: 3.5 needed no Inspector edit — it already calls `formatSizeLine(image)` with the
whole record, so `durationMs` flows through the widened `SizeFacts`.
Lightbox test stubs `ResizeObserver` and `clientWidth/Height` so the viewport measures; the
image click then requests an animation frame and the video click does not.
Hand check: 3.6 is open. Watch that Space closes with focus on the surface (a click on the
video's controls moves focus into the video, where Space is the platform's), that the badge
is legible over the hover caption gradient (it sits under it on hover), and that a
rotated/odd-aspect clip fits the window.

### U1

- `scripts/build-ffmpeg.sh [triple]`: `$1` is the Tauri target triple, default the `host:` line of
  `rustc -vV`. Output `packages/app/src-tauri/binaries/ffmpeg-<triple>[.exe]`, plus
  `ffmpeg-<triple>.stamp` (the script's own sha256) and `binaries/LICENSE.ffmpeg` (LGPL 2.1
  text, in `bundle.resources`). Binary, licence and matching stamp present: prints "up to date",
  exits 0, downloads nothing. Editing the script rebuilds.
- Deviation from D2: `--enable-zlib` added. With `--disable-autodetect` the png encoder is
  compiled out ("Unknown encoder 'png'") without zlib. macOS links the system libz; the Windows
  step installs `mingw-w64-x86_64-zlib` and links it statically. Binary: 3.0 MB (arm64).
- Windows steps (both workflows) assume: `msys2/setup-msys2@v2` MINGW64 with gcc, zlib, make,
  diffutils; `shell: msys2 {0}`; triple passed as `x86_64-pc-windows-msvc`; curl, tar (xz) and
  strip come with MSYS2. Untested until task 1.6 dispatches the run. macOS step passes
  `aarch64-apple-darwin`. Both cache `binaries/` keyed on runner OS and the script hash.
- `build.rs` emits `BOORUBOX_TARGET` for U2's `cfg(test)` lookup: `env!("BOORUBOX_TARGET")`.
- `cargo build`/`cargo test` fail without the binary; run `mise run ffmpeg` first.

### U3 review fixes

- Hand check: click on the video and on its controls, then press arrows, Space, `i`, `e`: the keys keep working (pointer release hands focus back to the surface).
- Hand check: trash a clip that cannot be played; the next video is tried, not pre-marked failed. A write (trash, collection, artist edit) no longer restarts the playing video or drops unmute/seek.
- Hand check: on a video tile the duration badge lifts above the caption while the tile is hovered or keyboard-current.

### U2

Landed (gate green: `cargo test` 868, clippy, lint, `tauri build --no-bundle`): `media.rs`
(`probe`), `ffmpeg.rs` (`locate`, `first_frame`), `error.rs` (`Unsupported(String)`, also 422 in
`http::error_response`), `ingest.rs` (`media::probe` replaces `decode`; `image_columns()` and
`row_to_record` gain `duration_ms` as column 20, after `source_url`), `import.rs`, `db.rs`
(`SCHEMA_V15`), `model.rs`, `sidecar.rs`, `recover.rs`, `thumbs.rs`, `lib.rs`.
- Migration: **v15** (`MIGRATIONS.len()` was 14). D3's sentence amended.
- `Probed { kind: Kind::{Image,Video}, ext, mime, width: u32, height: u32, duration_ms:
  Option<i64>, codec: Option<&'static str> }`. `probe` tries `image::guess_format` first; a
  non-image with `ftyp` at byte 4 goes to the mp4 path, one starting `1A 45 DF A3` to the webm
  path, anything else returns the image crate's `Decode` error.
- Reason strings, exact: `video codec mp4v is not supported` (also `av01`, `vp08`, `s263`, and
  `unknown` for a sample entry the parser cannot name), `no video track`,
  `matroska container is not supported, only webm`, `unreadable mp4 header: <debug>`,
  `unreadable webm header: <error>`. Matroska refusals use the codec id as the file names it,
  e.g. `video codec V_AV1 is not supported`.
- Deviation from D1: `mp4parse` 0.17, not `mp4` 0.14. `mp4` drops any sample entry it does not
  know (`mp4v`, `av01`, `hvc1`, `avc3`), so a refusal cannot name the codec. `mp4parse` names
  H.264 (`avc1`/`avc3`), MP4V, VP8, VP9, AV1, H.263 but has no HEVC: an `hvc1`/`hev1` mp4 is
  refused as `video codec unknown is not supported`. **HEVC in mp4 is therefore not accepted
  yet**; the spec's accepted table says it should be. The right shape is a parser that names
  HEVC, or a fourcc read of the `stsd` entry; not built (the brief forbids a hand-written box
  parser). Webm accepts HEVC and AVC ids as designed. mp4 duration is the video track's `mdhd`
  duration over its timescale (the movie header's is not exposed); rounded, 200 ms on both
  fixtures. Webm DocType is checked by finding the text `webm` in the first 64 bytes, because
  the `matroska` crate skips the EBML header.
- Sidecar binary: `current_exe().parent()/ffmpeg[.exe]` in the app; under `cfg(test)`
  `$CARGO_MANIFEST_DIR/binaries/ffmpeg-$BOORUBOX_TARGET[.exe]`. A missing binary is an `Io` error,
  so a poster failure is swallowed after import and counted `failed` by regenerate.
- Smoke test: watch that the `.app`/`target/debug` really carries `ffmpeg` beside the executable
  (tests never exercise `current_exe()`), that an iPhone-style `hvc1` clip is refused as
  `video codec unknown`, and that a video with a rotation matrix shows swapped Size dimensions.
- `write_through_part` takes the mime and picks the decoder; `ensure_thumbnail` and
  `regenerate_all` pass `record.mime`.

### U1/U2 review fixes

- `build-ffmpeg.sh` exports `MACOSX_DEPLOYMENT_TARGET=11.0` for `*apple-darwin`; the CI mac
  sidecar now has minos 11.0 instead of the runner's. The script changed, so stamps rebuild.
- The release body says the portable `boorubox.exe` shows no video thumbnails (no `ffmpeg.exe`
  beside it); design D2 records why. Hand check: the portable exe imports and plays a video.
- A zero duration (fragmented mp4, webm without one) is `None`, so no `0:00` badge; a zero or
  out-of-range video dimension is refused as `Unsupported`. mp4/webm signatures are tested
  before the image sniffer.
