// Canvas capture of an image the page has already loaded. Ported from the
// legacy extension's `src/content/index.ts` (requirements §4: port this code,
// do not rewrite it), with design D13's one departure: the bytes leave as a
// data URL.
//
// Extension messages are JSON-serialised, so a `Blob` in a response arrives as
// `{}`. Returning one is why the legacy canvas path never actually ran — every
// capture it made came from the background's fetch fallback. Change this back
// and the same silent dead branch comes with it.

/** What the background gets back for a `CAPTURE_IMAGE` message. */
export type CaptureResult
  = | { dataUrl: string, width: number, height: number }
    | { error: string }

export async function captureImage(imageUrl: string): Promise<CaptureResult> {
  const img = findImageElement(imageUrl)
  if (!img) {
    return { error: 'Image not found in DOM' }
  }
  try {
    return {
      dataUrl: await captureImageAsDataUrl(img),
      width: img.naturalWidth,
      height: img.naturalHeight,
    }
  } catch (error) {
    return { error: error instanceof Error ? error.message : 'Unknown error' }
  }
}

/**
 * The clicked image, matched three ways in order: exactly, then normalised
 * against the page URL, then with the query string dropped — sites serve the
 * same file under cache-busting parameters, and the context menu reports the
 * URL the browser fetched, not the one in the markup.
 */
export function findImageElement(url: string): HTMLImageElement | null {
  const images = document.querySelectorAll('img')

  for (const img of images) {
    if (img.src === url || img.currentSrc === url) {
      return img
    }
  }

  try {
    const normalizedUrl = new URL(url, window.location.href).href

    for (const img of images) {
      try {
        const imgSrc = img.src || img.currentSrc
        if (!imgSrc) continue

        const imgUrl = new URL(imgSrc, window.location.href).href
        if (imgUrl === normalizedUrl) {
          return img
        }
        if (imgUrl.split('?')[0] === normalizedUrl.split('?')[0]) {
          return img
        }
      } catch {
        continue
      }
    }
  } catch {
    // The clicked URL will not parse; the exact pass above was the only chance.
  }

  return null
}

export async function captureImageAsDataUrl(img: HTMLImageElement): Promise<string> {
  if (!img.complete) {
    await new Promise((resolve, reject) => {
      img.onload = resolve
      img.onerror = reject
      setTimeout(() => reject(new Error('Image load timeout')), 5000)
    })
  }

  if (img.naturalWidth === 0 || img.naturalHeight === 0) {
    throw new Error('Image has no dimensions')
  }

  const canvas = document.createElement('canvas')
  canvas.width = img.naturalWidth
  canvas.height = img.naturalHeight

  const ctx = canvas.getContext('2d')
  if (!ctx) {
    throw new Error('Failed to get canvas context')
  }
  ctx.drawImage(img, 0, 0)

  return canvas.toDataURL()
}
