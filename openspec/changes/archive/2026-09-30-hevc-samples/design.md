## Context

`hevc-remux` gave the viewer `canDecode(mime, codec, probe)` and a refusal message; the
sidecar has the mp4 muxer; `thumbs.rs` is the model for a derived cache (`.thumbs/<a1>/<id>.jpg`,
`ensure_thumbnail` on demand, deleted in `trash.rs`, the asset scope granted per directory in
`commands.rs::grant_asset_scope`). ffmpeg's `-progress <file|pipe:1> -nostats` prints
`out_time_us=` lines about once a second. Measured on the owner's 75 s 1080p clip:
`h264_videotoolbox` at 6 Mbps takes 27 s and gives 59 MB (the source is 50 MB).

## Decisions

### D1 — What a sample is and where it lives

`.samples/<a1>/<id>.mp4` (`library::shard_dirs`, the bucket rule thumbnails use), H.264
`avc1` in mp4 with `faststart`, AAC audio, made from the stored file by the sidecar:
`-nostdin -loglevel error -y -i <stored> -c:v <encoder> -b:v <bitrate> -pix_fmt yuv420p
-tag:v avc1 -c:a aac -b:a 128k -movflags +faststart -progress pipe:1 -nostats -f mp4
<part>`, then rename `part` → the sample. The encoder is `h264_mf` on Windows and
`h264_videotoolbox` on macOS (a `cfg!`); the bitrate is the source's (size × 8 / duration)
clamped to 2–12 Mbps, so a sample is about the size of its source and never a re-encode of
a bad guess. Written through a `.mp4.part` in the same directory that the sweep on open removes when stray.
The sample is a cache (design §7 of `video-files`: derived, deletable): `trash.rs` and the
drop path remove it with the thumbnail; a Settings → Library "Clear playback samples" button
deletes the directory's files and reports the count and bytes freed; `relayout` ignores it.

### D2 — Made only when the engine cannot decode the original, and only on request

Rust never decides whether a machine needs a sample; the webview does, with `canDecode`
(`hevc-remux` D3). The viewer calls `playback_sample(id)` only when `canDecode` is false.
The command returns the existing sample's path at once, else encodes and returns when done.
Encodes are serialised behind one process-wide mutex so two clips opened in a row do not
race two encoders; a request for a clip already encoding waits on the same mutex and finds
the file. The library lock is not held during the encode (the command reads the record and
the paths under the lock, then releases it, as `thumbnail_path` does). FIXME on the command:
no cancel — moving on leaves the encode running to completion (cached for next time); the
right shape is a cancellation token checked between progress lines. Not built: a conversion
is a one-off per clip.

### D3 — Progress and the message

The command streams progress as Tauri events `playback-sample-progress` `{ id, ratio }`
(`out_time_us` / `duration_ms × 1000`, clamped to 0–1) parsed from ffmpeg's stdout lines; a
final `{ id, ratio: 1 }` is implied by the command's return. The viewer's state for an
undecodable codec becomes: request the sample, show "Converting this video for playback:
this machine's browser engine cannot decode HEVC (H.265). This takes about as long as the
clip. Please wait…" (the codec name from `codecName`; on Windows the sentence about
Microsoft's HEVC Video Extensions stays, so the user knows why), a progress bar bound to the
event's ratio, then the `<video>` with the sample's URL (`convertFileSrc`, the scope grants
`.samples/` beside `.thumbs/`). A failed encode shows the refusal message from `hevc-remux`
plus ffmpeg's reason. Moving to another item while converting shows that item; the encode
finishes in the background. The keys work throughout.

### D4 — The sidecar build

`scripts/build-ffmpeg.sh`: `--enable-encoder=png,aac,h264_videotoolbox --enable-videotoolbox`
on `*apple-darwin`; `--enable-encoder=png,aac,h264_mf --enable-mediafoundation` on
`*windows*`; both: `--enable-decoder=…,aac,mp3,ac3 --enable-parser=…,aac,mpegaudio,ac3
--enable-filter=…,aformat,aresample`. `--disable-autodetect` stays, so the two frameworks are
named explicitly. The Windows step's MSYS2 MinGW headers carry Media Foundation; if configure
refuses `--enable-mediafoundation` there, the unit stops and says so — that is the one risk.

## Risks

- Media Foundation's software H.264 encoder is slower than VideoToolbox: expect one to two
  minutes for a 75 s 1080p clip on a typical laptop. The message says to wait; the bar shows
  progress.
- A machine whose MF encoder is missing or refuses the input: the viewer shows the refusal
  with ffmpeg's reason, as today.
