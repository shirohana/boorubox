// The heading a group key is shown as. Rust returns the raw key — the account
// handle, or `WIDTHxHEIGHT-SIZE` — because a display string is UI and the
// storage layer writes none (design D7). This is that UI, kept out of the
// component so the shapes it has to read can be asserted.

import type { GroupBy } from '@boorubox/shared'
import { formatBytes } from '$lib/domain/format'

const DUPLICATE_KEY = /^(\d+)x(\d+)-(\d+)$/

export function groupLabel(group: GroupBy, key: string): string {
  if (group === 'x-account') return `@${key}`
  if (group !== 'duplicates') return key

  const parts = DUPLICATE_KEY.exec(key)
  // A key that does not parse is shown as it came: a wrong-looking heading is a
  // fact about the data, and inventing one would hide it.
  if (!parts) return key
  return `${parts[1]} × ${parts[2]} · ${formatBytes(Number(parts[3]))}`
}
