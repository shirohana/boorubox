/**
 * Rebuild the bytes a content script sent as a data URL (design D13,
 * departure 1).
 *
 * Base64 only: the producer is `canvas.toDataURL()`, which never emits the
 * percent-encoded form.
 */
export function dataUrlToBlob(dataUrl: string): Blob {
  const comma = dataUrl.indexOf(',')
  if (!dataUrl.startsWith('data:') || comma === -1) {
    throw new Error('not a data URL')
  }
  const mime = dataUrl.slice('data:'.length, comma).split(';')[0] || 'application/octet-stream'
  const binary = atob(dataUrl.slice(comma + 1))
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }
  return new Blob([bytes], { type: mime })
}
