// The picture on a history row: 128 px on the longest edge, JPEG at 0.7,
// carried inside the entry as a data URL (design D3).
//
// It is decoration for a 40-px row on a 2x display, and at roughly 4 KB per
// entry a full history costs about a megabyte — which is what lets the popup
// render every row from one storage read.

export const THUMB_EDGE = 128
const QUALITY = 0.7

/**
 * A thumbnail for these bytes, or an empty string if one cannot be made.
 *
 * Never raises. A capture is not worth losing over its decoration, and this
 * runs before the bytes have been handed to anyone (§2 guarantee 3).
 */
export async function makeThumbnail(blob: Blob): Promise<string> {
  try {
    const bitmap = await createImageBitmap(blob)
    try {
      const scale = Math.min(1, THUMB_EDGE / Math.max(bitmap.width, bitmap.height))
      const canvas = new OffscreenCanvas(
        Math.max(1, Math.round(bitmap.width * scale)),
        Math.max(1, Math.round(bitmap.height * scale)),
      )
      const ctx = canvas.getContext('2d')
      if (!ctx) {
        return ''
      }
      ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height)
      const jpeg = await canvas.convertToBlob({ type: 'image/jpeg', quality: QUALITY })
      return await toDataUrl(jpeg)
    } finally {
      bitmap.close()
    }
  } catch {
    return ''
  }
}

async function toDataUrl(blob: Blob): Promise<string> {
  const bytes = new Uint8Array(await blob.arrayBuffer())
  let binary = ''
  for (const byte of bytes) {
    binary += String.fromCharCode(byte)
  }
  return `data:${blob.type};base64,${btoa(binary)}`
}
