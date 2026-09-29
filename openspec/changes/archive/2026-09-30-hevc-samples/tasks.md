> Three units. U1 (sidecar encoders) lands first and is proven on both platforms before U2;
> U3 (viewer) runs beside U1 with the command mocked. Design D1–D4 decide every shape. Gate:
> `mise run check`. No unit commits or ticks a hand check.

## 1. Unit U1 — sidecar encoders (`scripts/`) — Sonnet

- [x] 1.1 `scripts/build-ffmpeg.sh` per D4 (per-triple encoder and framework flags, the
      audio decoders/parsers/filters); header comment updated (why the OS encoders and not
      libx264: LGPL, no bundled library; why per platform).
- [x] 1.2 Gate: `mise run ffmpeg` rebuilds; the sidecar encodes
      `packages/app/src-tauri/fixtures/video/hvc1.mp4` with `-c:v h264_videotoolbox -b:v 2M
      -pix_fmt yuv420p -tag:v avc1 -c:a aac -movflags +faststart -progress pipe:1 -nostats
      -f mp4 <tmp>` and the system ffprobe reports `avc1`; the poster pipe and the remux still
      work; `mise run lint`; report the size.
- [x] 1.3 (lead) The Windows build: push the branch, `windows-test-build.yml` green with the
      MSYS2 configure accepting `--enable-mediafoundation`. Not a hand check: the log.
      Evidence: run 36602011254 on `f6c0f9d`, success; configure listed `mediafoundation` and
      the step logged `built …/ffmpeg-x86_64-pc-windows-msvc.exe`.

## 2. Unit U2 — samples in Rust (`packages/app/src-tauri`) — Sonnet, after 1.3

- [x] 2.1 `samples.rs` per D1–D3: `sample_path`, `part_path`, `ensure_sample(paths, record,
      on_progress)` (the mutex, the bitrate rule, the progress parse, part-then-rename,
      cleanup on error), `clear_all(paths) -> SamplesReport { removed, bytes }`. `ffmpeg.rs`
      gains `encode_sample(source, dest, encoder, bitrate, on_progress)` sharing
      `extractor_command`/`check_status`; the FIXME per D2.
- [x] 2.2 `commands.rs`: `playback_sample(id) -> SampleRef { path, version }` emitting
      `playback-sample-progress` events (read how `thumbnail_path` releases the lock and
      versions the file); `clear_playback_samples() -> SamplesReport`; `grant_asset_scope`
      adds `samples_dir()`; `library.rs`: `samples_dir()` beside `thumbs_dir()`, created on
      open like `.thumbs/`, ignored by relayout; `trash.rs` and the drop path remove the
      sample with the thumbnail.
- [x] 2.3 Tests (macOS, the dev machine): `ensure_sample_of_the_hvc1_fixture_is_avc1_with_
      the_same_size_and_length`, `a_second_call_returns_the_cached_file`,
      `progress_is_reported_between_zero_and_one`, `clear_all_reports_count_and_bytes`,
      `trashing_a_video_removes_its_sample`, `the_bitrate_rule_clamps`.
- [x] 2.4 Gate green; handoff (command shapes, event name and payload, the part name).
- [ ] 2.5 Hand check (owner, Windows): open the `hvc1` clip — the converting message, a
      moving bar, then playback; reopen — instant; Settings → Clear playback samples reports
      one file; reopen — converts again.

## 3. Unit U3 — the viewer converts (`packages/app/src`) — Sonnet, beside U1

- [x] 3.1 `lib/api/commands.ts`: `playbackSample(id)`, `clearPlaybackSamples()`; the event
      listener helper for `playback-sample-progress` (read how other events are listened to).
- [x] 3.2 `Lightbox.svelte` per D3: the converting state (message via `codecName`, the Windows
      sentence, a progress bar), the sample URL when ready, the failure message with the
      reason, moving on while converting; `$derived`, no store-field `$effect`.
- [x] 3.3 Settings → Library: "Clear playback samples" with its report line (read how
      "Regenerate thumbnails" is presented and mirror it).
- [x] 3.4 Component tests: undecodable codec → the command is called once, the message and bar
      render, a progress event moves the bar, the resolved path mounts a `<video>` with that
      URL; a decodable codec never calls the command; a rejected command shows the reason.
      Gate (`pnpm lint`, `pnpm typecheck`, `pnpm test`); handoff.
- [ ] 3.5 Hand check (owner, Mac): nothing converts (the original plays); Settings shows the
      button and reports zero.

## Handoff

### U1

- `scripts/build-ffmpeg.sh` rebuilt (aarch64-apple-darwin); binary 3804680 bytes.
- Encode that worked: `ffmpeg -nostdin -loglevel error -y -i <src> -c:v h264_videotoolbox -b:v 2M
  -pix_fmt yuv420p -tag:v avc1 -c:a aac -movflags +faststart -progress pipe:1 -nostats -f mp4 <dest>`
  (`-b:v 6M` for the 75 s clip). Progress lines `out_time_us=` arrive on stdout.
- hvc1 fixture: 1 progress line, `h264|avc1`. 75 s clip: 11.9 s wall (about 6x realtime), 58930933
  bytes at 6M, ffprobe `h264|avc1` and `aac|mp4a`. Poster pipe 651 bytes; hev1 remux exits 0.
- `mise run lint` fails only in U3's files (Lightbox.svelte operator-linebreak error; two
  max-len warnings in events.ts, codecs.ts); the script is not linted.

### U2

- Command `playback_sample { id }` -> `{ path, version }` (version = file mtime ms); rejects with the
  error string. Command `clear_playback_samples {}` -> `{ removed, bytes }`. Event
  `playback-sample-progress` `{ id, ratio }`, one per ffmpeg `out_time_us` line (about 1/s, not
  throttled further). All camelCase; structs `SampleRef`, `SamplesReport`, `SampleProgress` in model.rs.
- Files: `.samples/<a1>/<id>.mp4`, written via `<id>.mp4.part` in the same directory. Nothing sweeps
  `.samples/` on open (only the inbox and the thumbs relayout sweep `.part`); `clear_all` removes stray
  parts too. `ffmpeg::encode_sample` takes an extra `duration_ms` argument (for the ratio).
- Bitrate: `size * 8 * 1000 / duration_ms` clamped to 2-12 Mbps; null or zero duration -> 6 Mbps.
  Encoder `h264_mf` on Windows, `h264_videotoolbox` elsewhere. One process-wide mutex serialises encodes.
- Progress test uses the real fixture: VideoToolbox printed a line. Parsing is also unit-tested
  (`ffmpeg::tests::progress_lines_give_a_clamped_ratio`).
- Windows hand check: does `h264_mf` accept `-pix_fmt yuv420p -b:v` and produce avc1 with faststart;
  does the bar move (MF may print few progress lines); is the part file flushed and renamed (no sharing
  violation from the webview reading it); Clear reports the right count.

### review fixes

- Clear playback samples leaves a running encode alone: `clear_all` skips `*.part` and counts only
  finished samples, so a Clear during a conversion no longer breaks it (Windows sharing violation).
- `Library::open_or_create` now sweeps stray `.samples/<a1>/*.part`. Re-opening the same library while
  an encode runs unlinks that encode's part; the encode then fails and the user can retry.
- A record deleted forever mid-encode has its finished sample removed and the call rejects "no such record".
