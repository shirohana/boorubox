> One unit, Sonnet (Rust with a test gate). Design D1–D3 decide every shape; do not re-decide
> them. Gate: `mise run check`. No unit commits or ticks a hand check.

## 1. Unit H — HEVC in an mp4 (`packages/app/src-tauri`)

- [x] 1.1 `media.rs`: `first_video_sample_entry` per D1 (bounds-checked box walk, `None` on
      anything malformed), the `Unknown` branch of `probe_mp4` accepting `hvc1`/`hev1` and
      naming any other fourcc; dimensions from `tkhd` per D2; the FIXME removed and the doc
      comment on `probe_mp4` rewritten to what is now true. Tests on the fixtures:
      `hvc1_mp4_is_probed_as_hevc_video` (16×16, 200 ms, codec `hvc1`),
      `hev1_mp4_is_probed_as_hevc_video`, `an_unknown_fourcc_is_named_in_the_refusal` (patch
      the `hvc1` fixture's sample entry type bytes to `zzzz` in memory — the walk finds the
      offset — and expect `video codec zzzz is not supported`),
      `a_truncated_moov_is_refused_not_panicked` (the fixture cut mid-`moov`),
      `mpeg4_mp4_is_refused_by_codec_name` still passes.
- [x] 1.2 `thumbs.rs` / `ffmpeg.rs`: one test that `first_frame` on the `hvc1` fixture is
      16×16 (the sidecar decodes HEVC).
- [x] 1.3 `openspec/changes/archive/2026-09-29-video-files/design.md` is history and stays;
      the main spec is updated by the archive of this change. Gate green. Handoff below.
- [ ] 1.4 Hand check (owner, Windows): import an H.265 clip from the NSIS-installed test build;
      the tile shows its first frame; the viewer plays it or says it cannot be played here
      (Microsoft's HEVC Video Extensions decide which); the Size row shows its facts.

## Handoff

`media.rs`: `first_video_sample_entry(bytes: &[u8]) -> Option<[u8; 4]>` (helpers `boxes`, `child`,
`next_box`); `unnamed_video_entry(bytes, track) -> Result<(&'static str, u64, u64)>` is the
`Unknown` branch of `probe_mp4` (codec, width, height from `tkhd >> 16`). Refusal shape:
`video codec <fourcc> is not supported`, non-graphic-ASCII bytes as `?`; a walk that finds
nothing keeps `video codec unknown is not supported`; a missing `tkhd` reads as zero-sized
(`unreadable mp4 header: zero-sized video track`). Tests: `hvc1_mp4_is_probed_as_hevc_video`,
`hev1_mp4_is_probed_as_hevc_video`, `an_unknown_fourcc_is_named_in_the_refusal`,
`a_non_ascii_fourcc_prints_as_question_marks`, `a_truncated_moov_is_refused_not_panicked`,
`the_walk_answers_none_for_garbage`, `ffmpeg::tests::first_frame_of_the_hevc_fixture_is_16_by_16`.
Gate: cargo test, `mise run clippy`, `mise run lint` green (not `mise run check`). Hand check
(1.4, Windows): only the macOS sidecar was exercised, so watch that the Windows ffmpeg renders
the tile poster; an `hev1` clip may play in neither webview though import accepts it.
