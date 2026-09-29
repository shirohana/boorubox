> Three units. U1 (sidecar flags) lands first; U2 (Rust) needs U1's binary; U3 (viewer) runs
> beside U1 after the lead's U0 (the shared `codec` field). Design D1–D4 decide every shape.
> The migration number is `MIGRATIONS.len()` when U2 lands (15 today, planned v16): amend
> D2's sentence to the real number. Gate: `mise run check`. No unit commits or ticks a hand
> check.

## 0. Unit U0 — shared type (lead)

- [x] 0.1 `packages/shared/src/index.ts`: `codec?: string | null` on `ImageRecord` (the
      container's four-letter code, videos only). Commit.

## 1. Unit U1 — sidecar mp4 muxer (`scripts/`, `.github`) — Sonnet

- [x] 1.1 `scripts/build-ffmpeg.sh`: `--enable-muxer=image2pipe,mp4` and
      `--enable-bsf=hevc_mp4toannexb,extract_extradata` per D4; header comment says why.
- [x] 1.2 Gate: `mise run ffmpeg` rebuilds; `./packages/app/src-tauri/binaries/ffmpeg-aarch64-
      apple-darwin -nostdin -loglevel error -i packages/app/src-tauri/fixtures/video/hev1.mp4
      -c copy -tag:v hvc1 -movflags +faststart -f mp4 /tmp/x.mp4` exits 0 and `ffprobe`
      (the system one) reports `hvc1`; report the binary size; `mise run lint`.

## 2. Unit U2 — remux, codec column (`packages/app/src-tauri`) — Sonnet, after U1

- [x] 2.1 `media.rs`: `Probed.codec` for every mp4 comes from the walk (`avc1`/`avc3`/`hvc1`/
      `hev1`), webm ids mapped per D2; tests: `avc3_is_told_from_avc1` (patch the h264
      fixture's entry type to `avc3` in memory), `vp9_webm_reports_vp09`.
- [x] 2.2 `ffmpeg.rs`: `remux(source, dest, tag)` per D1 with the same spawn shape as
      `first_frame`; test: the `hev1` fixture remuxed to a temp file is probed as `hvc1`
      with the same dimensions and duration.
- [x] 2.3 `ingest.rs`: the remux step between the inbox write and the rename per D1, the
      re-probe from disk, the size from the stored file; a remux error fails the import with
      ffmpeg's last line. Tests: `an_hev1_mp4_is_stored_as_hvc1` (stored file probed as
      `hvc1`, row codec `hvc1`, the input bytes untouched), `an_avc1_mp4_is_stored_as_is`
      (bytes identical), `a_remux_failure_leaves_no_file_and_no_row` (a truncated hev1 that
      probes but ffmpeg rejects — if none can be made, say so in the handoff).
- [x] 2.4 `db.rs` migration per D2; `model.rs`, `sidecar.rs`, `recover.rs`: `codec`; tests in
      the existing style (migration from the previous version; a sidecar with `codec` restores
      it, one without restores null).
- [x] 2.5 Gate green; handoff (real migration number, the remux command line, reason strings).
- [ ] 2.6 Hand check (owner, Mac): import `boorubox-vault/h265-test-video.mp4` (hev1); it
      plays in the viewer; the inspector's file is the same size give or take the header.

## 3. Unit U3 — the viewer asks first (`packages/app/src`) — Sonnet, beside U1

- [x] 3.1 `lib/domain/codecs.ts`: `codecsString(mime, codec)` and `canDecode(mime, codec,
      probe)` where `probe` is `canPlayType` injected for tests; tests for the four codecs
      and for null (always "try").
- [x] 3.2 `Lightbox.svelte` per D3: the check before mounting the `<video>`, the message
      naming the codec, the Windows sentence via `$lib/platform`; component tests: a record
      with `codec: 'hvc1'` and a probe answering `''` renders the message and no `<video>`;
      a probe answering `probably` renders the `<video>`; null codec renders the `<video>`.
- [x] 3.3 Gate (`pnpm lint`, `pnpm typecheck`, `pnpm test`); handoff.
- [ ] 3.4 Hand check (owner, Windows): the `hvc1` clip shows the HEVC message, no sound.

## Handoff

### U1

Built with `--enable-muxer=image2pipe,mp4 --enable-bsf=hevc_mp4toannexb,extract_extradata`;
binary 3,201,304 bytes (arm64). Remux command that worked (ffprobe reports `hvc1`; the h264
fixture with `-tag:v avc1` reports `avc1`):
`ffmpeg -nostdin -loglevel error -i <src> -c copy -tag:v hvc1 -movflags +faststart -f mp4 <dest>`
The poster pipe still works. `mise run lint` fails only on another unit's
`Lightbox.svelte` (arrow-parens, indent), not on this script.

### U3

`isWindows` sits beside `isMacos` in `lib/platform.ts`; `app.html` now stamps `data-platform="windows"` from the user agent. Windows hand check (3.4): with an `hvc1` clip and no HEVC Video Extensions the viewer must show "This machine's browser engine cannot decode HEVC (H.265). Windows' engine plays HEVC only with Microsoft's HEVC Video Extensions." with no sound, arrows still moving; with the extension installed the clip must play (the probe answers non-empty). Also confirm `data-platform` reads `windows` in the Windows webview (UA contains "Windows").

### U2

Migration is v16 (`SCHEMA_V16`, `ALTER TABLE images ADD COLUMN codec TEXT`, `MIGRATIONS.len()` 16). Remux command line
(`ffmpeg::remux`): `ffmpeg -nostdin -loglevel error -y -i <id>.part -c copy -tag:v <hvc1|avc1> -movflags +faststart -f mp4 <id>.remux.part`;
then `<id>.remux.part` is renamed over `<id>.part` and the part is re-read and probed (`ingest::remux_in_place`). Both names end in
`.part`, so `Library::sweep_inbox` covers the second. Errors: ffmpeg non-zero is `AppError::Io("ffmpeg failed (<status>): <last stderr line>")`
(shared `check_status` with `first_frame`); a rewrite whose re-probe fails is `AppError::Io("the rewritten mp4 is unreadable: <probe error>")`.
Both parts are removed and no row is written. `a_remux_failure_leaves_no_file_and_no_row` cannot use a truncated file (the fixture's
moov is at the end) and ffmpeg's stream copy exits 0 on garbage samples, so the input is the hev1 fixture with its `stco` offset moved
past EOF: ffmpeg logs "partial file", exits 0, writes a 30-byte file with no video track, and the re-probe error is what fails the
import. Hand check for 2.6 stays open.
