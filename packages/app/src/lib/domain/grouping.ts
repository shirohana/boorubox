import type { ImageRecord } from '@boorubox/shared'

export type GroupBy = 'none' | 'x-account' | 'duplicates'

/**
 * FIXME: two implementations of one rule. The `account:` filter runs in SQL
 * through the `x_account()` scalar function in src-tauri/src/query.rs; this is
 * the same extraction in TypeScript, for grouping in the webview. Change one
 * without the other and `account:alice` and the X-account groups disagree.
 */
export function getXAccountFromUrl(url: string | null): string | null {
  if (!url) return null
  try {
    const urlObj = new URL(url)
    // Match x.com or twitter.com
    if (urlObj.hostname === 'x.com' || urlObj.hostname === 'twitter.com'
      || urlObj.hostname === 'www.x.com' || urlObj.hostname === 'www.twitter.com') {
      // Extract account from path like /accountname/status/...
      const match = urlObj.pathname.match(/^\/([^/]+)/)
      if (match && match[1]) {
        // Skip non-account paths
        const path = match[1].toLowerCase()
        if (path === 'i' || path === 'home' || path === 'explore'
          || path === 'notifications' || path === 'messages' || path === 'search') {
          return null
        }
        return match[1]
      }
    }
    return null
  } catch {
    return null
  }
}

export function groupImagesByXAccount(images: ImageRecord[]): Map<string, ImageRecord[]> {
  const groups = new Map<string, ImageRecord[]>()

  for (const image of images) {
    const account = getXAccountFromUrl(image.pageUrl)
    if (account) {
      if (!groups.has(account)) {
        groups.set(account, [])
      }
      groups.get(account)!.push(image)
    }
  }

  return groups
}

export function groupImagesByDuplicates(images: ImageRecord[]): Map<string, ImageRecord[]> {
  const groups = new Map<string, ImageRecord[]>()

  for (const image of images) {
    // Group by dimensions AND file size
    const key = `${image.width}×${image.height}-${image.size}`
    if (!groups.has(key)) {
      groups.set(key, [])
    }
    groups.get(key)!.push(image)
  }

  // Only return groups with 2+ images (actual duplicates)
  const duplicates = new Map<string, ImageRecord[]>()
  for (const [key, groupImages] of groups) {
    if (groupImages.length >= 2) {
      duplicates.set(key, groupImages)
    }
  }

  return duplicates
}

/**
 * Returns images in the order they visually appear on screen for the given grouping.
 * Used by selection/navigation so keyboard indices match the rendered layout.
 * Pure: pass the filtered images and current groupBy in rather than reading globals.
 */
export function getVisualOrder(images: ImageRecord[], groupBy: GroupBy): ImageRecord[] {
  if (groupBy === 'none') {
    // Ungrouped: use filtered images as-is
    return images
  } else if (groupBy === 'x-account') {
    // Group by X account: sort by count (desc) then alphabetically
    const groups = groupImagesByXAccount(images)
    const sortedAccounts = Array.from(groups.entries())
      .sort((a, b) => {
        const countDiff = b[1].length - a[1].length
        if (countDiff !== 0) return countDiff
        return a[0].localeCompare(b[0])
      })
      .map(([account]) => account)

    const visualOrder: ImageRecord[] = []
    for (const account of sortedAccounts) {
      visualOrder.push(...groups.get(account)!)
    }
    return visualOrder
  } else if (groupBy === 'duplicates') {
    // Group by duplicates: sort by key alphabetically
    const groups = groupImagesByDuplicates(images)
    const sortedKeys = Array.from(groups.keys()).sort()

    const visualOrder: ImageRecord[] = []
    for (const key of sortedKeys) {
      visualOrder.push(...groups.get(key)!)
    }
    return visualOrder
  }

  // Fallback
  return images
}
