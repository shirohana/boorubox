#!/usr/bin/env bash
# Builds the minimal ffmpeg the app spawns to take a video's first frame, and writes it where
# Tauri looks up a sidecar: packages/app/src-tauri/binaries/ffmpeg-<target triple>[.exe].
#
# Usage: scripts/build-ffmpeg.sh [target-triple]
#   The triple defaults to the `host:` line of `rustc -vV`. Workflows pass it explicitly.
#
# A stamp file (`ffmpeg-<triple>.stamp`) beside the binary holds this script's own sha256; a
# matching stamp and an existing binary and licence text mean nothing is downloaded or built.
# Editing this script (a flag, the version) invalidates the stamp, so the binary can never be
# older than its recipe.
#
# The configure line, which is also the LGPL's "how it was built":
#   --disable-everything --disable-doc --disable-ffprobe --disable-ffplay --disable-network
#   --disable-autodetect --disable-avdevice --disable-postproc --disable-debug --disable-x86asm
#   --enable-small --enable-zlib --enable-protocol=file,pipe --enable-demuxer=mov,matroska
#   --enable-decoder=h264,hevc,vp8,vp9 --enable-parser=h264,hevc,vp8,vp9
#   --enable-encoder=png --enable-muxer=image2pipe,mp4 --enable-bsf=hevc_mp4toannexb,extract_extradata
#   --enable-filter=scale,select,format
#
# The mp4 muxer and the two bitstream filters exist for a stream copy that moves in-band
# parameter sets (`hev1`/`avc3`) into the container (`hvc1`/`avc1`), so WebKit plays the file.
# `hevc-remux` design D4.
#
# Why that line: the four codecs (h264, hevc, vp8, vp9) are the ones the webviews play, so a
# file the app can show is a file ffmpeg can take a frame from; everything else stays off to
# keep the binary small. No `--enable-gpl` and no external library, so the result is LGPL 2.1
# and ships as its own process. `--disable-x86asm` so no nasm is needed: one keyframe per file
# does not pay for SIMD. `--enable-zlib` because ffmpeg's png encoder is compiled out
# without it ("Unknown encoder 'png'" at run time); zlib is not GPL. To add a decoder, add it to `--enable-decoder`, its parser to
# `--enable-parser` and, for a new container, its demuxer to `--enable-demuxer`.
set -euo pipefail

VERSION=7.1.1
SHA256=733984395e0dbbe5c046abda2dc49a5544e7e0e1e2366bba849222ae9e3a03b1
URL="https://ffmpeg.org/releases/ffmpeg-$VERSION.tar.xz"

root=$(cd "$(dirname "$0")/.." && pwd)
script="$root/scripts/build-ffmpeg.sh"
out_dir="$root/packages/app/src-tauri/binaries"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

triple=${1:-$(rustc -vV | sed -n 's/^host: //p')}
exe=""
case "$triple" in *windows*) exe=".exe" ;; esac
binary="$out_dir/ffmpeg-$triple$exe"
stamp="$out_dir/ffmpeg-$triple.stamp"
licence="$out_dir/LICENSE.ffmpeg"
script_hash=$(sha256_of "$script")

if [ -f "$binary" ] && [ -f "$licence" ] && [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$script_hash" ]; then
  echo "ffmpeg-$triple$exe is up to date"
  exit 0
fi

mkdir -p "$out_dir"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

curl -fsSL --retry 3 -o "$work/ffmpeg.tar.xz" "$URL"
if [ "$(sha256_of "$work/ffmpeg.tar.xz")" != "$SHA256" ]; then
  echo "ffmpeg-$VERSION.tar.xz does not match its pinned sha256" >&2
  exit 1
fi
tar -xJf "$work/ffmpeg.tar.xz" -C "$work"
src="$work/ffmpeg-$VERSION"

flags=(
  --disable-everything --disable-doc --disable-ffprobe --disable-ffplay --disable-network
  --disable-autodetect --disable-avdevice --disable-postproc --disable-debug --disable-x86asm
  --enable-small --enable-zlib
  --enable-protocol=file,pipe
  --enable-demuxer=mov,matroska
  --enable-decoder=h264,hevc,vp8,vp9
  --enable-parser=h264,hevc,vp8,vp9
  --enable-encoder=png
  --enable-muxer=image2pipe,mp4
  --enable-bsf=hevc_mp4toannexb,extract_extradata
  --enable-filter=scale,select,format
)
# Under MSYS2 a MinGW build links its runtime as DLLs by default; the app ships one file, so
# link everything in.
if [ -n "${MSYSTEM:-}" ]; then
  flags+=(--extra-ldflags=-static --pkg-config-flags=--static)
fi

# Without a deployment target the binary's minimum macOS is the build host's, and on an older
# macOS it will not launch, so every poster would read "No preview". 11.0 is the floor of
# aarch64-apple-darwin, the lowest the app itself targets.
case "$triple" in *apple-darwin) export MACOSX_DEPLOYMENT_TARGET=11.0 ;; esac

jobs=$(nproc 2>/dev/null || sysctl -n hw.ncpu)
(cd "$src" && ./configure "${flags[@]}" && make -j"$jobs")

strip "$src/ffmpeg$exe" 2>/dev/null || strip -x "$src/ffmpeg$exe"
# Tauri looks a sidecar up by the Rust host triple, so the MinGW-built file is named for the
# MSVC triple; the name says nothing about the compiler that made it.
cp "$src/ffmpeg$exe" "$binary"
cp "$src/COPYING.LGPLv2.1" "$licence"
echo "$script_hash" > "$stamp"
echo "built $binary"
