/** Codec code (the sample entry's fourcc) to a `canPlayType` codecs parameter. */
const CODECS_PARAMETER: Record<string, string> = {
  hvc1: 'hvc1.1.6.L93.B0',
  avc1: 'avc1.42E01E',
  vp08: 'vp8',
  vp09: 'vp09.00.10.08',
}

const CODEC_NAMES: Record<string, string> = {
  hvc1: 'HEVC (H.265)',
  avc1: 'H.264',
  vp08: 'VP8',
  vp09: 'VP9',
}

/** The type string `canPlayType` takes for a container and a recorded codec. */
export function codecsString(mime: string, codec: string): string {
  return `${mime}; codecs="${CODECS_PARAMETER[codec] ?? codec}"`
}

/** What a person reads for a codec code; an unknown code is shown as it is. */
export function codecName(codec: string): string {
  return CODEC_NAMES[codec] ?? codec
}

/**
 * Whether the engine will decode a video of this codec. Nothing recorded means
 * try it; of `canPlayType`'s answers only the empty string refuses, since
 * `'maybe'` is the engine's honest "probably".
 */
export function canDecode(
  mime: string,
  codec: string | null | undefined,
  probe: (type: string) => string,
): boolean {
  if (!codec) return true
  return probe(codecsString(mime, codec)) !== ''
}

const WINDOWS_HEVC_SENTENCE = 'Windows\' engine plays HEVC only with Microsoft\'s HEVC Video Extensions.'

/** The engine's refusal of `codec`, from "browser engine" on, with the Windows note where it applies. */
function engineCannotDecode(codec: string, windows: boolean): string {
  const base = `browser engine cannot decode ${codecName(codec)}.`
  return windows && codec === 'hvc1' ? `${base} ${WINDOWS_HEVC_SENTENCE}` : base
}

/** What the viewer says in a picture's place when the engine will not decode `codec`. */
export function refusalMessage(codec: string, windows: boolean): string {
  return `This machine's ${engineCannotDecode(codec, windows)}`
}

/** What the viewer says while it converts a video the engine will not decode. */
export function convertingMessage(codec: string, windows: boolean): string {
  const why = engineCannotDecode(codec, windows)
  return `Converting this video for playback: this machine's ${why} This takes about as long as the clip. Please wait…`
}
